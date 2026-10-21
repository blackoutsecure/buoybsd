// SPDX-License-Identifier: Apache-2.0

package cli

import (
	"context"
	"errors"
	"flag"
	"fmt"
	"io"
	"log/slog"
	"os"
	"runtime"

	"github.com/blackoutsecure/int-buoybsd/internal/collector/replay"
	"github.com/blackoutsecure/int-buoybsd/internal/detect"
	"github.com/blackoutsecure/int-buoybsd/internal/output"
	"github.com/blackoutsecure/int-buoybsd/internal/pipeline"
	"github.com/blackoutsecure/int-buoybsd/internal/policy"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
)

func RunSensor(ctx context.Context, args []string, stdout, stderr io.Writer) error {
	if stdout == nil || stderr == nil {
		return errors.New("sensor requires output and diagnostic writers")
	}
	flags := flag.NewFlagSet("buoybsd", flag.ContinueOnError)
	flags.SetOutput(stderr)
	configPath := flags.String("config", "", "YAML configuration file")
	replayPath := flags.String("replay", "", "synthetic replay NDJSON file; no live collector is implemented")
	format := flags.String("output", "json", "stdout format: json or table")
	version := flags.Bool("version", false, "print version without collecting")
	if err := flags.Parse(args); err != nil {
		if errors.Is(err, flag.ErrHelp) {
			return nil
		}
		return err
	}
	if flags.NArg() != 0 {
		return errors.New("buoybsd does not accept positional arguments")
	}
	if *version {
		_, err := fmt.Fprintf(stdout, "buoybsd %s (%s/%s; replay prototype)\n", event.Version, runtime.GOOS, runtime.GOARCH)
		return err
	}
	cfg := config.Default()
	if *configPath != "" {
		var err error
		cfg, err = loadConfig(*configPath)
		if err != nil {
			return err
		}
	}
	var overrideErr error
	flags.Visit(func(value *flag.Flag) {
		switch value.Name {
		case "replay":
			cfg.Collector.Path = *replayPath
		case "output":
			if *format != "json" && *format != "table" {
				overrideErr = errors.New("output must be json or table")
				return
			}
			replaced := false
			for i := range cfg.Outputs {
				if cfg.Outputs[i].Type == "stdout" {
					cfg.Outputs[i].Format = *format
					replaced = true
				}
			}
			if !replaced {
				cfg.Outputs = append(cfg.Outputs, config.Destination{Name: "cli-stdout", Type: "stdout", Format: *format})
			}
		}
	})
	if overrideErr != nil {
		return overrideErr
	}
	if err := cfg.Validate(); err != nil {
		return fmt.Errorf("validate configuration: %w", err)
	}
	if err := cfg.ValidatePaths(*configPath); err != nil {
		return fmt.Errorf("validate paths: %w", err)
	}
	policies, err := policy.Compile(cfg.Policies)
	if err != nil {
		return err
	}
	file, err := os.Open(cfg.Collector.Path)
	if err != nil {
		return fmt.Errorf("open replay input: %w", err)
	}
	source, err := replay.New(file, cfg.Collector.MaxRecordBytes)
	if err != nil {
		return errors.Join(err, file.Close())
	}
	destinations, err := openDestinations(cfg.Outputs, stdout)
	if err != nil {
		return errors.Join(err, source.Close())
	}
	var detectors []detect.Detector
	if len(cfg.Detectors) != 0 {
		detectors = append(detectors, detect.ShellExec{})
	}
	logger := slog.New(slog.NewJSONHandler(stderr, nil))
	stats, err := pipeline.Run(ctx, source, destinations, pipeline.Options{
		QueueSize: cfg.Pipeline.QueueSize, Policies: policies, Detectors: detectors,
	})
	level := slog.LevelInfo
	if err != nil {
		level = slog.LevelError
	}
	logger.Log(ctx, level, "pipeline stopped", "component", "pipeline", "collector", source.Name(), "metrics", stats)
	return err
}

func loadConfig(path string) (cfg config.Config, result error) {
	info, err := os.Stat(path)
	if err != nil {
		return cfg, fmt.Errorf("stat configuration: %w", err)
	}
	if !info.Mode().IsRegular() {
		return cfg, errors.New("configuration must be a regular file")
	}
	file, err := os.Open(path)
	if err != nil {
		return cfg, fmt.Errorf("open configuration: %w", err)
	}
	defer func() { result = errors.Join(result, file.Close()) }()
	return config.Load(file)
}

func openDestinations(specs []config.Destination, stdout io.Writer) (destinations []output.Destination, result error) {
	defer func() {
		if result != nil {
			for _, destination := range destinations {
				result = errors.Join(result, destination.Close())
			}
			destinations = nil
		}
	}()
	for _, spec := range specs {
		var destination output.Destination
		var err error
		switch spec.Type {
		case "stdout":
			if spec.Format == "table" {
				destination, err = output.NewTable(spec.Name, stdout)
			} else {
				destination, err = output.NewJSON(spec.Name, stdout, nil)
			}
		case "file":
			var file *os.File
			file, err = os.OpenFile(spec.Path, os.O_CREATE|os.O_APPEND|os.O_WRONLY, 0600)
			if err == nil {
				destination, err = output.NewJSON(spec.Name, file, file)
				if err != nil {
					err = errors.Join(err, file.Close())
				}
			}
		case "webhook":
			destination, err = output.NewWebhook(spec)
		default:
			err = errors.New("unsupported destination type")
		}
		if err != nil {
			return destinations, fmt.Errorf("open destination %q: %w", spec.Name, err)
		}
		destinations = append(destinations, destination)
	}
	return destinations, nil
}
