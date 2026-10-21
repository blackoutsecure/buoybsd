// SPDX-License-Identifier: Apache-2.0

package output

import (
	"context"
	"errors"
	"fmt"
	"io"
	"strconv"
	"time"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
)

type Table struct {
	counters
	name   string
	writer io.Writer
	header bool
}

func NewTable(name string, writer io.Writer) (*Table, error) {
	if name == "" || writer == nil {
		return nil, errors.New("table output requires a name and writer")
	}
	return &Table{name: name, writer: writer}, nil
}

func (t *Table) Name() string { return t.name }
func (t *Table) Close() error { return nil }

func (t *Table) WriteEvent(ctx context.Context, event *v1.Event) error {
	t.attempts.Add(1)
	if err := ctx.Err(); err != nil {
		return t.complete(err)
	}
	if event == nil || event.Timestamp == nil {
		return t.complete(errors.New("table output requires an event timestamp"))
	}
	if err := event.Timestamp.CheckValid(); err != nil {
		return t.complete(errors.New("invalid table event timestamp"))
	}
	if !t.header {
		if err := write(t.writer, []byte("TIME\tEVENT\tPID\tEXECUTABLE\n")); err != nil {
			return t.complete(err)
		}
		t.header = true
	}
	pid, executable := "-", "-"
	if event.Process != nil {
		if event.Process.Pid != nil {
			pid = strconv.FormatUint(uint64(*event.Process.Pid), 10)
		}
		if event.Process.ExecutablePath != nil {
			executable = strconv.QuoteToASCII(*event.Process.ExecutablePath)
		}
	}
	line := fmt.Sprintf("%s\t%s\t%s\t%s\n",
		event.Timestamp.AsTime().UTC().Format(time.RFC3339Nano),
		strconv.QuoteToASCII(event.Name), pid, executable)
	return t.complete(write(t.writer, []byte(line)))
}
