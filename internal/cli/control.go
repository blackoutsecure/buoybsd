// SPDX-License-Identifier: Apache-2.0

package cli

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"

	"github.com/blackoutsecure/int-buoybsd/pkg/event"
)

func RunControl(args []string, stdout io.Writer) error {
	if stdout == nil {
		return errors.New("control client requires an output writer")
	}
	if len(args) == 1 && (args[0] == "version" || args[0] == "--version") {
		_, err := fmt.Fprintln(stdout, "buoyctl "+event.Version)
		return err
	}
	if len(args) == 2 && args[0] == "events" && args[1] == "list" {
		return json.NewEncoder(stdout).Encode(event.Definitions())
	}
	if len(args) == 1 && args[0] == "status" {
		return errors.New("status unavailable: no live collector or control service is implemented")
	}
	return errors.New("usage: buoyctl version | events list | status (not implemented)")
}
