package main

import (
	"encoding/json"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/hairyhenderson/go-which"
	"github.com/integrii/flaggy"
)

type Dependency struct {
	Name     string `json:"name"`
	URL      string `json:"url"`
	Revision string `json:"revision"`
}

type FabCache struct {
	Prefix       string            `json:"prefix"`
	Dependencies []Dependency      `json:"dependencies"`
	Options      map[string]string `json:"options"`
	Install      map[string]string `json:"install"`
}

const VERSION = "1.0.0"

const CACHE_FILENAME = "fabricate_cache.json"

func (dependency Dependency) String() string {
	return fmt.Sprintf("Dependency(%s)", dependency.Name)
}

func main() {
	// Find Ninja
	ninjaPath := which.Which("ninja")
	if ninjaPath == "" {
		panic(fmt.Errorf("could not locate \"ninja\""))
	}

	// Setup Main
	parser := flaggy.NewParser("fabricate")
	parser.Version = VERSION

	buildDir := os.Getenv("BUILDDIR")
	if buildDir == "" {
		buildDir = "build"
	}
	parser.String(&buildDir, "", "builddir", "Specify the build directory path (default: ./build) [Environment Variable: BUILDDIR].")

	// Setup Configure
	configureCommand := flaggy.NewSubcommand("configure")
	configureCommand.Description = "Configures the build directory with the given arguments"
	parser.AttachSubcommand(configureCommand, 1)

	options := make([]string, 0)
	configureCommand.StringSlice(&options, "o", "option", "Specify the value of a *user defined* option in the format of key=value.")

	prefix := "/usr"
	configureCommand.String(&prefix, "", "prefix", "Specify installation prefix (default: /usr).")

	configPath := "fab.lua"
	configureCommand.String(&configPath, "", "config", "Specify the configuration file path (default: fab.lua).")

	// Setup Build
	buildCommand := flaggy.NewSubcommand("build")
	buildCommand.Description = "Build the project"
	parser.AttachSubcommand(buildCommand, 1)

	// Setup Install
	installCommand := flaggy.NewSubcommand("install")
	installCommand.Description = "Install built files"
	parser.AttachSubcommand(installCommand, 1)

	destdir := os.Getenv("DESTDIR")
	installCommand.String(&destdir, "", "destdir", "Specify the destdir of the install [Environment Variable: DESTDIR].")

	// Parse
	parser.Parse()

	var err error

	if configPath, err = filepath.Abs(configPath); err != nil {
		panic(err)
	}

	if buildDir, err = filepath.Abs(buildDir); err != nil {
		panic(err)
	}

	// Load cache
	cache := FabCache{
		Dependencies: make([]Dependency, 0),
		Install:      make(map[string]string),
	}

	if cacheData, err := os.ReadFile(filepath.Join(buildDir, CACHE_FILENAME)); err != nil {
		if !os.IsNotExist(err) {
			panic(err)
		}
	} else {
		if err := json.Unmarshal(cacheData, &cache); err != nil {
			panic(err)
		}
	}

	// Execute
	switch parser.TrailingSubcommand().Name {
	case "fabricate":
		parser.ShowHelpWithMessage("Missing subcommand")

	case "configure":
		optionsMap := make(map[string]string, 0)
		for _, option := range options {
			parts := strings.SplitN(option, "=", 2)
			if len(parts) != 2 {
				panic(fmt.Errorf("invalid option format `%s` (expected key=value)", option))
			}
			optionsMap[parts[0]] = parts[1]
		}

		if err = configure(cache, ninjaPath, configPath, buildDir, optionsMap, prefix); err != nil {
			panic(err)
		}

	case "build":
		buildCmd := exec.Command(ninjaPath, "-C", buildDir)
		buildCmd.Stdout = os.Stdout
		buildCmd.Stderr = os.Stderr

		if err := buildCmd.Run(); err != nil {
			panic(err)
		}

	case "install":
		for dest, src := range cache.Install {
			dest = filepath.Join(prefix, dest)
			src = filepath.Join(buildDir, src)

			if destdir != "" {
				dest = filepath.Join(destdir, dest)
			}

			info, err := os.Stat(src)
			if os.IsNotExist(err) {
				panic(fmt.Errorf("no such output `%s`", src))
			}

			if info.IsDir() {
				panic(fmt.Errorf("output `%s` is a directory", src))
			}

			if err := os.MkdirAll(filepath.Dir(dest), 0755); err != nil {
				panic(err)
			}

			srcFile, err := os.OpenFile(src, os.O_RDONLY, 0755)
			if err != nil {
				panic(err)
			}

			destFile, err := os.OpenFile(dest, os.O_WRONLY|os.O_CREATE, 0755)
			if err != nil {
				panic(err)
			}

			if _, err := io.Copy(destFile, srcFile); err != nil {
				panic(err)
			}
		}
	}
}
