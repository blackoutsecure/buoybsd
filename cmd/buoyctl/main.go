// SPDX-License-Identifier: Apache-2.0

package main

import (
	"log/slog"
	"os"

	"github.com/blackoutsecure/int-buoybsd/internal/cli"
)

func main() {
	if err := cli.RunControl(os.Args[1:], os.Stdout); err != nil {
		logger := slog.New(slog.NewJSONHandler(os.Stderr, nil))
		logger.Error("control request failed", "component", "cli", "operation", "control", "error", err)
		os.Exit(1)
	}
}
