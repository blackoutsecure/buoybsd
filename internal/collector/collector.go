// SPDX-License-Identifier: Apache-2.0

package collector

import (
	"context"
	"time"
)

const (
	MinRecordBytes = 256
	MaxRecordBytes = 1 << 20
	MaxFieldBytes  = 4096
	MaxArguments   = 128
)

type Collector interface {
	Name() string
	Run(context.Context, chan<- RawEvent) error
	Close() error
	Stats() Stats
}

type Stats struct {
	Received     uint64
	Dropped      uint64
	ParserErrors uint64
	Restarts     uint64
}

type RawEvent struct {
	Name            string
	Timestamp       *time.Time
	Collector       string
	OperatingSystem string
	Evidence        string
	SourceID        *string
	Sequence        uint64
	Host            *Host
	Process         *Process
	Jail            *Jail
	Exec            *ProcessExec
	Record          []byte
	MediaType       string
}

type Host struct {
	Name *string `json:"name"`
}

type Process struct {
	PID            *uint32 `json:"pid"`
	TID            *uint32 `json:"tid"`
	ParentPID      *uint32 `json:"ppid"`
	UID            *uint32 `json:"uid"`
	EUID           *uint32 `json:"euid"`
	GID            *uint32 `json:"gid"`
	EGID           *uint32 `json:"egid"`
	ExecutablePath *string `json:"executable_path"`
	Command        *string `json:"command"`
}

type Jail struct {
	ID   *uint32 `json:"id"`
	Name *string `json:"name"`
}

type ProcessExec struct {
	Argv         []string `json:"argv"`
	ArgvComplete *bool    `json:"argv_complete"`
	Success      *bool    `json:"success"`
}
