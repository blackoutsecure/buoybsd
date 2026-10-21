// SPDX-License-Identifier: Apache-2.0

package main

import (
	"context"
	"log/slog"
	"os"
	"os/signal"
	"syscall"

	"github.com/blackoutsecure/int-buoybsd/internal/cli"
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer stop()
	if err := cli.RunSensor(ctx, os.Args[1:], os.Stdout, os.Stderr); err != nil {
		logger := slog.New(slog.NewJSONHandler(os.Stderr, nil))
		logger.Error("sensor failed", "component", "cli", "operation", "run", "error", err)
		os.Exit(1)
	}
}
