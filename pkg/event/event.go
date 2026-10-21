// SPDX-License-Identifier: Apache-2.0

package event

import v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"

const (
	SchemaVersion = "buoybsd.v1beta1"
	ReplayVersion = "buoybsd.replay.v1"
	ProcessExec   = "process_exec"
	Version       = "0.1.0-dev"
)

type Definition struct {
	ID        v1.EventId `json:"id"`
	Name      string     `json:"name"`
	Collector string     `json:"collector"`
	Status    string     `json:"status"`
}

func Definitions() []Definition {
	return []Definition{{
		ID:        v1.EventId_EVENT_ID_PROCESS_EXEC,
		Name:      ProcessExec,
		Collector: "replay",
		Status:    "synthetic replay only; native telemetry unverified",
	}}
}

func Known(name string) bool {
	return name == ProcessExec
}
