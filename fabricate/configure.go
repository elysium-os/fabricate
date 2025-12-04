package main

/*
 * This code is a crime against humanity.
 * DO NOT READ IT.
 * I BEG YOU, YOU MIGHT NOT SURVIVE TO TELL THE TALE.
 */

import (
	"bytes"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"slices"
	"strconv"
	"strings"

	"github.com/Shopify/go-lua"
	"github.com/bmatcuk/doublestar"
	"github.com/go-git/go-git/v5"
	"github.com/go-git/go-git/v5/plumbing"
	"github.com/hairyhenderson/go-which"

	_ "embed"
)

const (
	DEPSTYLE_NORMAL = iota
	DEPSTYLE_GCC
	DEPSTYLE_MSVC
)

type DepStyle int

type Executable struct {
	path string // absolute
}

type Source struct {
	path string // absolute
}

type Rule struct {
	name        string
	description string // optional, empty string means not present
	command     string
	variables   []string
	depstyle    DepStyle
	doCompDB    bool
}

type Build struct {
	rule                 *Rule
	input                []string
	output               Output
	variables            map[string]string
	implicitDependencies []string
}

type Output struct {
	path string // relative to build directory
}

type FabConfiguration struct {
	projectRoot  string
	buildDir     string
	rules        []Rule
	builds       []Build
	outputs      []Output
	options      []string
	dependencies []Dependency
	installs     map[string]string
}

const OUTPUT_DIRNAME = "output"
const DEPFILES_DIRNAME = "depfiles"
const DEPENDENCIES_DIRNAME = "dependencies"

//go:embed lua/builtins.general.lua
var BUILTINS_GENERAL string

//go:embed lua/builtins.fab.lua
var BUILTINS_FAB string

var BUILTIN_VARIABLES = []string{"depfile"}
var RESERVED_VARIABLES = []string{"in", "out"}

func (executable Executable) String() string {
	return fmt.Sprintf("Executable(\"%s\")", executable.path)
}

func (source Source) String() string {
	return fmt.Sprintf("Source(\"%s\")", source.path)
}

func (rule Rule) String() string {
	return fmt.Sprintf("Rule(%s)", rule.name)
}

func (build Build) String() string {
	return fmt.Sprintf("Build(%s)", build.rule.name)
}

func (output Output) String() string {
	return fmt.Sprintf("Output(\"%s\")", output.path)
}

func configure(cache FabCache, ninjaPath string, configPath string, buildDir string, options map[string]string, prefix string, depdirs map[string]string) error {
	configDir := filepath.Dir(configPath)
	dependencyDir := filepath.Join(buildDir, "dependency")

	configuration := FabConfiguration{
		buildDir:    buildDir,
		projectRoot: configDir,
		installs:    make(map[string]string),
	}

	if err := os.Chdir(configDir); err != nil {
		panic(err)
	}

	// Generate Build Directory
	if err := os.MkdirAll(buildDir, 0755); err != nil {
		return err
	}

	if err := os.MkdirAll(filepath.Join(buildDir, OUTPUT_DIRNAME), 0755); err != nil {
		return err
	}

	if err := os.MkdirAll(filepath.Join(buildDir, DEPFILES_DIRNAME), 0755); err != nil {
		return err
	}

	// Cleanup builddir
	if cache.loaded {
		buildCmd := exec.Command(ninjaPath, "-C", buildDir, "-t", "cleandead")
		buildCmd.Stdout = os.Stdout
		buildCmd.Stderr = os.Stderr

		if err := buildCmd.Run(); err != nil {
			panic(err)
		}
	}

	// Setup Lua
	l := lua.NewState()
	lua.Require(l, "_G", lua.BaseOpen, true)
	lua.Require(l, "math", lua.MathOpen, true)
	lua.Require(l, "string", lua.StringOpen, true)
	lua.Require(l, "table", lua.TableOpen, true)
	lua.Require(l, "package", lua.PackageOpen, true)

	// Executable Metatable
	lua.NewMetaTable(l, "fab_executable")
	lua.SetFunctions(l, []lua.RegistryFunction{
		{Name: "__tostring", Function: func(l *lua.State) int {
			exe := lua.CheckUserData(l, 1, "fab_executable").(Executable)
			l.PushString(exe.String())
			return 1
		}},
		{Name: "__index", Function: func(l *lua.State) int {
			exe := lua.CheckUserData(l, 1, "fab_executable").(Executable)

			switch lua.CheckString(l, 2) {
			case "name":
				l.PushString(filepath.Base(exe.path))
			case "path":
				l.PushString(exe.path)
			case "invoke":
				l.PushGoFunction(func(l *lua.State) int {
					exe := lua.CheckUserData(l, 1, "fab_executable").(Executable)

					args := make([]string, 0)
					for i := 2; i <= l.Top(); i++ {
						args = append(args, lua.CheckString(l, i))
					}

					cmd := exec.Command(exe.path, args...)
					cmd.Stderr = os.Stderr

					data, err := cmd.Output()
					if err != nil {
						lua.Errorf(l, fmt.Sprintf("Invocation of %s failed: %s", exe, err))
					}

					l.PushString(string(data))
					return 1
				})
			default:
				l.PushNil()
			}

			return 1
		}},
	}, 0)
	l.Pop(1)

	// Source Metatable
	lua.NewMetaTable(l, "fab_source")
	lua.SetFunctions(l, []lua.RegistryFunction{
		{Name: "__tostring", Function: func(l *lua.State) int {
			source := lua.CheckUserData(l, 1, "fab_source").(Source)
			l.PushString(source.String())
			return 1
		}},
		{Name: "__index", Function: func(l *lua.State) int {
			source := lua.CheckUserData(l, 1, "fab_source").(Source)

			switch lua.CheckString(l, 2) {
			case "name":
				l.PushString(filepath.Base(source.path))
			case "path":
				l.PushString(source.path)
			default:
				l.PushNil()
			}

			return 1
		}},
	}, 0)
	l.Pop(1)

	// Rule Metatable
	lua.NewMetaTable(l, "fab_rule")
	lua.SetFunctions(l, []lua.RegistryFunction{
		{Name: "__tostring", Function: func(l *lua.State) int {
			rule := lua.CheckUserData(l, 1, "fab_rule").(Rule)
			l.PushString(rule.String())
			return 1
		}},
		{Name: "__index", Function: func(l *lua.State) int {
			rule := lua.CheckUserData(l, 1, "fab_rule").(Rule)

			switch lua.CheckString(l, 2) {
			case "name":
				l.PushString(rule.name)
			case "build":
				l.PushGoFunction(func(l *lua.State) int {
					rule := lua.CheckUserData(l, 1, "fab_rule").(Rule)
					out := lua.CheckString(l, 2)
					lua.CheckType(l, 3, lua.TypeTable)
					lua.CheckType(l, 4, lua.TypeTable)

					hasImplicits := !l.IsNoneOrNil(5)
					if hasImplicits {
						lua.CheckType(l, 5, lua.TypeTable)
					}

					out = filepath.Clean(out)
					if strings.HasPrefix(out, "..") || filepath.IsAbs(out) {
						lua.ArgumentError(l, 2, "output path escapes build directory")
					}
					out = filepath.Join("output", pathToFile(out))

					if slices.ContainsFunc(configuration.outputs, func(output Output) bool { return output.path == out }) {
						lua.ArgumentError(l, 2, fmt.Sprintf("build rule with the output path `%s` already exists", out))
					}

					in := make([]string, 0)
					l.PushNil()
					index := -3
					if hasImplicits {
						index = -4
					}
					for l.Next(index) {
						source := lua.TestUserData(l, -1, "fab_source")
						if source != nil {
							in = append(in, source.(Source).path)
							l.Pop(1)
							continue
						}

						output := lua.TestUserData(l, -1, "fab_output")
						if output != nil {
							in = append(in, output.(Output).path)
							l.Pop(1)
							continue
						}

						lua.ArgumentError(l, 3, fmt.Sprintf("input table contains an invalid value type `%s`", l.TypeOf(-1).String()))
					}

					variables := make(map[string]string, 0)
					l.PushNil()
					index = -2
					if hasImplicits {
						index = -3
					}
					for l.Next(-2) {
						key, ok := l.ToString(-2)
						if !ok {
							lua.ArgumentError(l, 4, fmt.Sprintf("variables contains an invalid key type `%s`", l.TypeOf(-2).String()))
						}

						value, ok := l.ToString(-1)
						if !ok {
							lua.ArgumentError(l, 4, fmt.Sprintf("variables contains an invalid value type `%s`", l.TypeOf(-1).String()))
						}
						l.Pop(1)

						if slices.Contains(RESERVED_VARIABLES, key) {
							lua.ArgumentError(l, 4, fmt.Sprintf("variables contains a reserved key `%s`", key))
						}

						if slices.Contains(BUILTIN_VARIABLES, key) {
							switch key {
							case "depfile":
								value = filepath.Clean(value)
								if strings.HasPrefix(value, "..") || filepath.IsAbs(value) {
									lua.ArgumentError(l, 2, "depfile path escapes depfiles directory")
								}
								value = filepath.Join("depfiles", pathToFile(value))
							}

							variables[key] = value
							continue
						}

						if !slices.Contains(rule.variables, key) {
							lua.ArgumentError(l, 4, fmt.Sprintf("variables contains an unknown key `%s`", key))
						}

						variables[fmt.Sprintf("fabvar_%s", key)] = value
					}

					implicitDependencies := make([]string, 0)
					if hasImplicits {
						l.PushNil()
						for l.Next(-2) {
							source := lua.TestUserData(l, -1, "fab_source")
							if source != nil {
								implicitDependencies = append(implicitDependencies, source.(Source).path)
								l.Pop(1)
								continue
							}

							output := lua.TestUserData(l, -1, "fab_output")
							if output != nil {
								implicitDependencies = append(implicitDependencies, output.(Output).path)
								l.Pop(1)
								continue
							}

							lua.ArgumentError(l, 3, fmt.Sprintf("implicitDependencies table contains an invalid value type `%s`", l.TypeOf(-1).String()))
						}
					}

					output := Output{path: filepath.Clean(filepath.Join(configuration.buildDir, out))}
					configuration.outputs = append(configuration.outputs, output)

					build := Build{rule: &rule, input: in, output: output, variables: variables, implicitDependencies: implicitDependencies}
					configuration.builds = append(configuration.builds, build)

					l.PushUserData(output)
					lua.MetaTableNamed(l, "fab_output")
					l.SetMetaTable(-2)
					return 1
				})
			default:
				l.PushNil()
			}

			return 1
		}},
	}, 0)
	l.Pop(1)

	// Output Metatable
	lua.NewMetaTable(l, "fab_output")
	lua.SetFunctions(l, []lua.RegistryFunction{
		{
			Name: "__tostring",
			Function: func(l *lua.State) int {
				output := lua.CheckUserData(l, 1, "fab_output").(Output)
				l.PushString(output.String())
				return 1
			},
		},
		{
			Name: "__index",
			Function: func(l *lua.State) int {
				output := lua.CheckUserData(l, 1, "fab_output").(Output)

				switch lua.CheckString(l, 2) {
				case "name":
					l.PushString(filepath.Base(output.path))
				case "path":
					l.PushString(output.path)
				case "install":
					l.PushGoFunction(func(l *lua.State) int {
						output := lua.CheckUserData(l, 1, "fab_output").(Output)
						dest := lua.CheckString(l, 2)

						if _, exists := configuration.installs[dest]; exists {
							lua.ArgumentError(l, 2, "Install path already used")
						}

						relativeOutput, err := filepath.Rel(buildDir, output.path)
						if err != nil {
							lua.Errorf(l, fmt.Sprintf("could not make relative path: %s", err))
						}

						configuration.installs[dest] = relativeOutput
						return 0
					})
				default:
					l.PushNil()
				}

				return 1
			},
		},
	}, 0)
	l.Pop(1)

	// Dependency Metatable
	lua.NewMetaTable(l, "fab_dependency")
	lua.SetFunctions(l, []lua.RegistryFunction{
		{
			Name: "__tostring",
			Function: func(l *lua.State) int {
				dependency := lua.CheckUserData(l, 1, "fab_dependency").(Dependency)
				l.PushString(dependency.String())
				return 1
			},
		},
		{
			Name: "__index",
			Function: func(l *lua.State) int {
				dependency := lua.CheckUserData(l, 1, "fab_dependency").(Dependency)

				switch lua.CheckString(l, 2) {
				case "name":
					l.PushString(dependency.Name)
				case "revision":
					l.PushString(dependency.Revision)
				case "url":
					l.PushString(dependency.URL)
				case "path":
					l.PushString(dependency.Path)
				case "glob":
					l.PushGoFunction(func(l *lua.State) int {
						ignores := make([]string, 0)
						for i := 3; i <= l.Top(); i++ {
							ignores = append(ignores, lua.CheckString(l, i))
						}

						matches, err := doGlob(dependency.Path, lua.CheckString(l, 2), ignores)
						if err != nil {
							lua.ArgumentError(l, 2, fmt.Sprintf("glob failed: %s", err))
						}

						l.NewTable()
						for i, v := range matches {
							l.PushNumber(float64(i + 1))
							l.PushString(v)
							l.SetTable(-3)
						}

						return 1
					})
				default:
					l.PushNil()
				}

				return 1
			},
		},
	}, 0)

	// Fab Table
	l.NewTable()
	lua.SetFunctions(l, []lua.RegistryFunction{
		{
			Name: "glob",
			Function: func(l *lua.State) int {
				ignores := make([]string, 0)
				for i := 2; i <= l.Top(); i++ {
					ignores = append(ignores, lua.CheckString(l, i))
				}

				matches, err := doGlob(configDir, lua.CheckString(l, 1), ignores)
				if err != nil {
					lua.ArgumentError(l, 1, fmt.Sprintf("glob failed: %s", err))
				}

				l.NewTable()
				for i, v := range matches {
					l.PushNumber(float64(i + 1))
					l.PushString(v)
					l.SetTable(-3)
				}

				return 1
			},
		},
		{
			Name: "path_join",
			Function: func(l *lua.State) int {
				args := make([]string, 0)
				for i := 1; i <= l.Top(); i++ {
					args = append(args, lua.CheckString(l, i))
				}
				l.PushString(filepath.Join(args...))
				return 1
			},
		},
		{
			Name: "path_abs",
			Function: func(l *lua.State) int {
				path, err := filepath.Abs(lua.CheckString(l, 1))
				if err != nil {
					lua.Errorf(l, fmt.Sprintf("Failed to make path absolute: %s", err))
				}
				l.PushString(path)
				return 1
			},
		},
		{
			Name: "string_split",
			Function: func(l *lua.State) int {
				str := lua.CheckString(l, 1)
				sep := lua.CheckString(l, 2)
				n, ok := l.ToInteger(3)
				if !ok {
					n = -1
				}

				l.NewTable()

				subs := strings.SplitN(str, sep, n)
				if subs == nil {
					return 1
				}

				for i, s := range subs {
					l.PushNumber(float64(i + 1))
					l.PushString(s)
					l.SetTable(-3)
				}
				return 1
			},
		},
		{
			Name: "path_rel",
			Function: func(l *lua.State) int {
				path, err := filepath.Abs(lua.CheckString(l, 1))
				if err != nil {
					lua.Errorf(l, fmt.Sprintf("Failed to make path absolute (in order to compute relative path): %s", err))
				}

				path, err = filepath.Rel(configuration.buildDir, path)
				if err != nil {
					lua.Errorf(l, fmt.Sprintf("Failed to make path relative: %s", err))
				}

				l.PushString(path)
				return 1
			},
		},
		{
			Name: "project_root",
			Function: func(l *lua.State) int {
				l.PushString(configuration.projectRoot)
				return 1
			},
		},
		{
			Name: "build_directory",
			Function: func(l *lua.State) int {
				l.PushString(configuration.buildDir)
				return 1
			},
		},
		{
			Name: "find_executable",
			Function: func(l *lua.State) int {
				found := which.Which(lua.CheckString(l, 1))

				if found == "" {
					l.PushNil()
				} else {
					l.PushUserData(Executable{path: found})
					lua.MetaTableNamed(l, "fab_executable")
					l.SetMetaTable(-2)
				}

				return 1
			},
		},
		{
			Name: "get_executable",
			Function: func(l *lua.State) int {
				path, ok := l.ToString(1)
				if ok {
					info, err := os.Stat(path)
					if os.IsNotExist(err) {
						lua.ArgumentError(l, 1, fmt.Sprintf("file `%s` does not exist", path))
					}

					if info.IsDir() {
						lua.ArgumentError(l, 1, fmt.Sprintf("`%s` is a directory", path))
					}
				} else {
					output := lua.TestUserData(l, 1, "fab_output")
					if output == nil {
						lua.ArgumentError(l, 1, "expected a string or output")
					}
					path = output.(Output).path
				}

				l.PushUserData(Executable{path})
				lua.MetaTableNamed(l, "fab_executable")
				l.SetMetaTable(-2)
				return 1
			},
		},
		{
			Name: "option",
			Function: func(l *lua.State) int {
				name := checkIdentifier(l, 1, lua.CheckString(l, 1))

				if slices.Contains(configuration.options, name) {
					lua.ArgumentError(l, 1, "option defined more than once")
				}
				configuration.options = append(configuration.options, name)

				required := l.ToBoolean(3)

				value, ok := options[name]
				if !ok {
					if required {
						lua.ArgumentError(l, 1, "no value provided")
					}

					l.PushNil()
					return 1
				}

				kind, ok := l.ToString(2)
				if ok {
					switch kind {
					case "string":
						l.PushString(value)
						return 1
					case "number":
						float, err := strconv.ParseFloat(value, 64)
						if err != nil {
							lua.Errorf(l, fmt.Sprintf("value `%s` is not a valid number: %s", value, err))
						}
						l.PushNumber(float)
						return 1
					}
				}

				if l.IsTable(2) {
					l.PushNil()
					for l.Next(2) {
						allowed, ok := l.ToString(-1)
						if !ok {
							lua.ArgumentError(l, 2, "not a list of strings")
						}

						if allowed == value {
							l.PushString(value)
							return 1
						}

						l.Pop(1)
					}
					l.Pop(1)

					lua.ArgumentError(l, 2, fmt.Sprintf("value `%s` is not in the combo", value))
				}

				lua.ArgumentError(l, 2, "invalid option kind")
				panic("unreachable")
			},
		},
		{
			Name: "source",
			Function: func(l *lua.State) int {
				path := lua.CheckString(l, 1)

				if !filepath.IsAbs(path) {
					path = filepath.Join(configDir, path)
				}

				inDep := false
				for _, depdir := range depdirs {
					if !pathIsWithin(l, path, depdir) {
						continue
					}
					inDep = true
				}

				if !pathIsWithin(l, path, configuration.projectRoot) && !pathIsWithin(l, path, dependencyDir) && !inDep {
					lua.ArgumentError(l, 1, "source is not within the project root or a dependency root")
				}

				l.PushUserData(Source{path: filepath.Clean(path)})
				lua.MetaTableNamed(l, "fab_source")
				l.SetMetaTable(-2)

				return 1
			},
		},
		{
			Name: "rule",
			Function: func(l *lua.State) int {
				lua.CheckType(l, 1, lua.TypeTable)

				variables := make([]string, 0)

				parseGeneric := func(l *lua.State) (string, bool) {
					var parts []string
					if l.IsTable(-1) {
						l.PushNil()
						for l.Next(-2) {
							value, ok := l.ToString(-1)
							if ok {
								parts = append(parts, value)
								l.Pop(1)
								continue
							}

							exe := lua.TestUserData(l, -1, "fab_executable")
							if exe != nil {
								parts = append(parts, exe.(Executable).path)
								l.Pop(1)
								continue
							}

							lua.ArgumentError(l, 1, fmt.Sprintf("command contains unsupported type `%s`", l.TypeOf(-1).String()))
						}
					} else {
						str, ok := l.ToString(-1)
						if !ok {
							return "", false
						}
						parts = strings.Fields(str)
					}

					escapedParts := make([]string, 0)
					for _, part := range parts {
						escapedParts = append(escapedParts, ninjaEscape(part))
					}

					regex, err := regexp.Compile("@.+?@")
					if err != nil {
						panic(err)
					}

					final := regex.ReplaceAllStringFunc(strings.Join(escapedParts, " "), func(variable string) string {
						variable = strings.ToLower(strings.Trim(variable, "@"))

						if slices.Contains(RESERVED_VARIABLES, variable) || slices.Contains(BUILTIN_VARIABLES, variable) {
							return fmt.Sprintf("$%s", variable)
						}

						variables = append(variables, variable)
						return fmt.Sprintf("$fabvar_%s", variable)
					})

					return final, true
				}

				// Name
				l.Field(1, "name")
				name, ok := l.ToString(-1)
				if !ok {
					lua.ArgumentError(l, 1, "missing or invalid value for \"name\"")
				}
				l.Pop(1)

				name = checkIdentifier(l, 1, name)

				if slices.ContainsFunc(configuration.rules, func(rule Rule) bool { return rule.name == name }) {
					lua.ArgumentError(l, 1, fmt.Sprintf("rule \"%s\" defined more than once", name))
				}

				// Command
				l.Field(1, "command")
				command, ok := parseGeneric(l)
				if !ok {
					lua.ArgumentError(l, 1, "missing or invalid value for \"command\"")
				}
				l.Pop(1)

				// Description
				l.Field(1, "description")
				var description string = ""
				if !l.IsNil(-1) {
					description, ok = parseGeneric(l)
					if !ok {
						lua.ArgumentError(l, 1, "invalid value for \"description\"")
					}
				}
				l.Pop(1)

				// Special Deps
				l.Field(1, "depstyle")
				var depstyle DepStyle = DEPSTYLE_NORMAL
				if !l.IsNil(-1) {
					style, ok := l.ToString(-1)
					if !ok {
						lua.ArgumentError(l, 1, "invalid value for \"depstyle\"")
					}

					switch style {
					case "normal":
						depstyle = DEPSTYLE_NORMAL
					case "gcc":
						fallthrough
					case "clang":
						depstyle = DEPSTYLE_GCC
					case "msvc":
						depstyle = DEPSTYLE_MSVC
					default:
						lua.ArgumentError(l, 1, "unknown value for \"depstyle\"")
					}
				}
				l.Pop(1)

				// CompDB
				l.Field(1, "compdb")
				doCompDB := l.ToBoolean(-1)
				l.Pop(1)

				// Create Rule
				rule := Rule{name, description, command, variables, depstyle, doCompDB}
				configuration.rules = append(configuration.rules, rule)

				l.PushUserData(rule)
				lua.MetaTableNamed(l, "fab_rule")
				l.SetMetaTable(-2)

				return 1
			},
		},
		{
			Name: "dependency",
			Function: func(l *lua.State) int {
				name := checkIdentifier(l, 1, lua.CheckString(l, 1))

				if slices.ContainsFunc(configuration.dependencies, func(d Dependency) bool { return d.Name == name }) {
					lua.ArgumentError(l, 1, fmt.Sprintf("dependency \"%s\" already exists", name))
				}

				url := lua.CheckString(l, 2)
				revision := lua.CheckString(l, 3)

				dependencyPath := filepath.Join(dependencyDir, name)

				depdir, hasDepDir := depdirs[name]
				if hasDepDir {
					dependencyPath = depdir
				}

				dependency := Dependency{name, url, revision, dependencyPath}
				configuration.dependencies = append(configuration.dependencies, dependency)

				for _, dep := range cache.Dependencies {
					if dep.Name != name {
						continue
					}

					if dep.URL != url || dep.Revision != revision || dep.Path != dependencyPath {
						break
					}

					l.PushUserData(dependency)
					lua.MetaTableNamed(l, "fab_dependency")
					l.SetMetaTable(-2)
					return 1
				}

				if !hasDepDir {
					os.RemoveAll(dependencyPath)
					repo, err := git.PlainClone(dependencyPath, false, &git.CloneOptions{
						URL:      url,
						Depth:    0,
						Progress: nil,
					})
					if err != nil {
						panic(err)
					}

					var ref *plumbing.Reference
					ref, err = repo.Reference(plumbing.NewBranchReferenceName(revision), true)
					if err != nil {
						ref, err = repo.Reference(plumbing.NewTagReferenceName(revision), true)
						if err != nil {
							hash := plumbing.NewHash(revision)
							_, err = repo.CommitObject(hash)
							if err != nil {
								log.Fatalf("Failed to resolve revision `%s`: %s", revision, err)
							}
							ref = plumbing.NewHashReference("", hash)
						}
					}

					wt, err := repo.Worktree()
					if err != nil {
						panic(err)
					}

					err = wt.Checkout(&git.CheckoutOptions{Hash: ref.Hash()})
					if err != nil {
						panic(err)
					}
				}

				l.PushUserData(dependency)
				lua.MetaTableNamed(l, "fab_dependency")
				l.SetMetaTable(-2)
				return 1
			},
		},
	}, 0)
	l.SetGlobal("fab")

	// Load builtins
	if err := lua.DoString(l, BUILTINS_GENERAL); err != nil {
		return err
	}

	if err := lua.DoString(l, BUILTINS_FAB); err != nil {
		return err
	}

	// Execute Config
	if err := lua.DoFile(l, configPath); err != nil {
		return fmt.Errorf("%s", err)
	}

	// Generate Fabricate Cache
	if cacheData, err := json.MarshalIndent(FabCache{
		Prefix:       prefix,
		Dependencies: configuration.dependencies,
		Options:      options,
		Install:      configuration.installs,
	}, "", "    "); err != nil {
		return err
	} else {
		if err := os.WriteFile(filepath.Join(buildDir, CACHE_FILENAME), cacheData, 0755); err != nil {
			return err
		}
	}

	// Generate .gitignore
	if err := os.WriteFile(filepath.Join(buildDir, ".gitignore"), []byte("# Generated by Fab.\n*"), 0755); err != nil {
		return err
	}

	// Generate ninja file
	data, err := ninjaGenerate(configuration)
	if err != nil {
		return err
	}

	if err := os.WriteFile(filepath.Join(buildDir, "build.ninja"), data, 0755); err != nil {
		return err
	}

	// Generate compile_commands file
	compDBRules := make([]string, 0)
	for _, rule := range configuration.rules {
		if !rule.doCompDB {
			continue
		}
		compDBRules = append(compDBRules, rule.name)
	}

	if len(compDBRules) > 0 {
		compileCommands, err := exec.Command(ninjaPath, slices.Concat([]string{"-C", buildDir, "-t", "compdb"}, compDBRules)...).Output()
		if err != nil {
			return err
		}

		if err := os.WriteFile(filepath.Join(buildDir, "compile_commands.json"), compileCommands, 0755); err != nil {
			return err
		}
	}

	return nil
}

func doGlob(path string, glob string, ignores []string) ([]string, error) {
	matches, err := doublestar.Glob(filepath.Join(path, glob))
	if err != nil {
		return nil, err
	}

	for _, ignore := range ignores {
		matches = slices.DeleteFunc(matches, func(m string) bool {
			match, err := doublestar.PathMatch(filepath.Join(path, ignore), m)
			if err != nil {
				return false
			}
			return match
		})
	}

	return matches, nil
}

func pathToFile(path string) string {
	return strings.Join(strings.Split(strings.ReplaceAll(path, "_", "__"), string(os.PathSeparator)), "_")
}

func pathIsWithin(l *lua.State, path string, in string) bool {
	relative, err := filepath.Rel(in, path)
	if err != nil {
		lua.Errorf(l, fmt.Sprintf("could not resolve relative path: %s", err))
	}

	return !strings.HasPrefix(relative, "..") && !filepath.IsAbs(relative)
}

func checkIdentifier(l *lua.State, argCount int, str string) string {
	if strings.HasPrefix(str, "fab_") {
		lua.ArgumentError(l, argCount, fmt.Sprintf("\"%s\" is an invalid identifier. cannot begin with \"fab_\"", str))
	}

	for _, ch := range str {
		if ch >= 'a' && ch <= 'z' {
			continue
		}

		if ch >= 'A' && ch <= 'Z' {
			continue
		}

		if ch == '-' || ch == '_' || ch == '.' {
			continue
		}

		lua.ArgumentError(l, argCount, fmt.Sprintf("\"%s\" is an invalid identifier. it is not alphabetic, '.', '_', or '-'", str))
	}

	return str
}

func ninjaEscape(str string) string {
	str = strings.ReplaceAll(str, "$", "$$")
	str = strings.ReplaceAll(str, " ", "$ ")
	str = strings.ReplaceAll(str, "\n", "$\n")
	return str
}

func ninjaGenerate(configuration FabConfiguration) ([]byte, error) {
	var buffer bytes.Buffer

	if _, err := buffer.WriteString("ninja_required_version = 1.9.0\n\n"); err != nil {
		return nil, err
	}

	if _, err := buffer.WriteString("# Rules\n"); err != nil {
		return nil, err
	}

	for _, rule := range configuration.rules {
		if _, err := buffer.WriteString(fmt.Sprintf("rule %s\n", rule.name)); err != nil {
			return nil, err
		}

		if _, err := buffer.WriteString(fmt.Sprintf("    command = %s\n", rule.command)); err != nil {
			return nil, err
		}

		if rule.description != "" {
			if _, err := buffer.WriteString(fmt.Sprintf("    description = %s\n", rule.description)); err != nil {
				return nil, err
			}
		}

		var depsValue string
		switch rule.depstyle {
		case DEPSTYLE_GCC:
			depsValue = "gcc"
		case DEPSTYLE_MSVC:
			depsValue = "msvc"
		}

		if depsValue != "" {
			if _, err := buffer.WriteString(fmt.Sprintf("    deps = %s\n", depsValue)); err != nil {
				return nil, err
			}
		}

		if _, err := buffer.WriteString("\n"); err != nil {
			panic(err)
		}
	}

	if _, err := buffer.WriteString("# Build Statements\n"); err != nil {
		panic(err)
	}

	for _, build := range configuration.builds {
		input := make([]string, 0)
		for _, inputPath := range build.input {
			relativePath, err := filepath.Rel(configuration.buildDir, inputPath)
			if err != nil {
				panic(fmt.Sprintf("could not resolve relative path to build directory: %s", err))
			}

			input = append(input, ninjaEscape(relativePath))
		}

		implicitDeps := make([]string, 0)
		for _, dep := range build.implicitDependencies {
			relativePath, err := filepath.Rel(configuration.buildDir, dep)
			if err != nil {
				panic(fmt.Sprintf("could not resolve relative path to build directory: %s", err))
			}

			implicitDeps = append(implicitDeps, ninjaEscape(relativePath))
		}

		output, err := filepath.Rel(configuration.buildDir, build.output.path)
		if err != nil {
			panic(fmt.Sprintf("could not resolve relative path to build directory: %s", err))
		}
		output = ninjaEscape(output)
		output = strings.ReplaceAll(output, ":", "$:")

		inputs := ""
		if len(input) > 0 {
			inputs += strings.Join(input, " ")
		}
		if len(build.implicitDependencies) > 0 {
			inputs = " | " + strings.Join(implicitDeps, " ")
		}

		if _, err := buffer.WriteString(fmt.Sprintf("build %s: %s %s\n", output, build.rule.name, inputs)); err != nil {
			return nil, err
		}

		for k, v := range build.variables {
			if _, err := buffer.WriteString(fmt.Sprintf("    %s = %s\n", k, ninjaEscape(v))); err != nil {
				panic(err)
			}
		}

		if _, err := buffer.WriteString("\n"); err != nil {
			panic(err)
		}
	}

	return buffer.Bytes(), nil
}
