// SPDX-License-Identifier: Apache-2.0

package output

import (
	"context"
	"errors"
	"io"
	"sync/atomic"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"google.golang.org/protobuf/encoding/protojson"
)

type Destination interface {
	Name() string
	WriteEvent(context.Context, *v1.Event) error
	Close() error
	Stats() Stats
}

type Stats struct {
	Attempts uint64
	Errors   uint64
}

type counters struct {
	attempts atomic.Uint64
	failures atomic.Uint64
}

func (c *counters) Stats() Stats {
	return Stats{Attempts: c.attempts.Load(), Errors: c.failures.Load()}
}

func (c *counters) complete(err error) error {
	if err != nil {
		c.failures.Add(1)
	}
	return err
}

func Marshal(event *v1.Event) ([]byte, error) {
	if event == nil {
		return nil, errors.New("output requires an event")
	}
	return (protojson.MarshalOptions{UseProtoNames: true}).Marshal(event)
}

func write(writer io.Writer, data []byte) error {
	n, err := writer.Write(data)
	if err != nil {
		return err
	}
	if n != len(data) {
		return io.ErrShortWrite
	}
	return nil
}
