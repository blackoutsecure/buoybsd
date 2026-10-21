// SPDX-License-Identifier: Apache-2.0

package output

import (
	"context"
	"errors"
	"io"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
)

type JSON struct {
	counters
	name   string
	writer io.Writer
	closer io.Closer
}

func NewJSON(name string, writer io.Writer, closer io.Closer) (*JSON, error) {
	if name == "" || writer == nil {
		return nil, errors.New("JSON output requires a name and writer")
	}
	return &JSON{name: name, writer: writer, closer: closer}, nil
}

func (j *JSON) Name() string { return j.name }

func (j *JSON) WriteEvent(ctx context.Context, event *v1.Event) error {
	j.attempts.Add(1)
	if err := ctx.Err(); err != nil {
		return j.complete(err)
	}
	data, err := Marshal(event)
	if err != nil {
		return j.complete(err)
	}
	return j.complete(write(j.writer, append(data, '\n')))
}

func (j *JSON) Close() error {
	if j.closer != nil {
		return j.closer.Close()
	}
	return nil
}
