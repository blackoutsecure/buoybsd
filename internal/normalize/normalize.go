// SPDX-License-Identifier: Apache-2.0

package normalize

import (
	"bytes"
	"crypto/sha256"
	"errors"
	"fmt"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/internal/collector"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func Event(raw collector.RawEvent) (*v1.Event, error) {
	if raw.Name != event.ProcessExec {
		return nil, errors.New("normalization: unsupported event")
	}
	if raw.Timestamp == nil {
		return nil, errors.New("normalization: source timestamp is required")
	}
	timestamp := timestamppb.New(*raw.Timestamp)
	if err := timestamp.CheckValid(); err != nil {
		return nil, errors.New("normalization: timestamp is outside protobuf range")
	}
	if len(raw.Record) == 0 || len(raw.Record) > collector.MaxRecordBytes {
		return nil, errors.New("normalization: raw record is missing or exceeds its limit")
	}
	if raw.Collector == "" || raw.OperatingSystem == "" || raw.Evidence == "" || raw.MediaType == "" {
		return nil, errors.New("normalization: source attribution is required")
	}
	hash := sha256.Sum256(raw.Record)
	result := &v1.Event{
		Timestamp:     timestamp,
		Id:            v1.EventId_EVENT_ID_PROCESS_EXEC,
		Name:          event.ProcessExec,
		SchemaVersion: event.SchemaVersion,
		RecordId:      fmt.Sprintf("sha256:%x", hash),
		Source: &v1.Source{
			Collector:       raw.Collector,
			OperatingSystem: raw.OperatingSystem,
			Evidence:        raw.Evidence,
			SourceId:        copyOptional(raw.SourceID),
			ReplaySequence:  raw.Sequence,
		},
		RawRecord: &v1.RawRecord{MediaType: raw.MediaType, Data: bytes.Clone(raw.Record)},
	}
	if raw.Host != nil {
		result.Host = &v1.Host{Name: copyOptional(raw.Host.Name)}
	}
	if raw.Process != nil {
		p := raw.Process
		result.Process = &v1.Process{
			Pid: copyOptional(p.PID), Tid: copyOptional(p.TID), ParentPid: copyOptional(p.ParentPID),
			Uid: copyOptional(p.UID), Euid: copyOptional(p.EUID), Gid: copyOptional(p.GID), Egid: copyOptional(p.EGID),
			ExecutablePath: copyOptional(p.ExecutablePath), Command: copyOptional(p.Command),
		}
	}
	if raw.Jail != nil {
		result.Jail = &v1.Jail{Id: copyOptional(raw.Jail.ID), Name: copyOptional(raw.Jail.Name)}
	}
	exec := &v1.ProcessExec{}
	if raw.Exec != nil {
		exec.Argv = append([]string(nil), raw.Exec.Argv...)
		exec.ArgvComplete = copyOptional(raw.Exec.ArgvComplete)
		exec.Success = copyOptional(raw.Exec.Success)
	}
	result.Data = &v1.Event_ProcessExec{ProcessExec: exec}
	if result.Process == nil || result.Process.Pid == nil {
		warn(result, "process.pid", "not observed")
	}
	if result.Process == nil || result.Process.ExecutablePath == nil {
		warn(result, "process.executable_path", "not observed")
	}
	if exec.ArgvComplete == nil || !*exec.ArgvComplete {
		warn(result, "process_exec.argv", "completeness not established")
	}
	if exec.Success == nil {
		warn(result, "process_exec.success", "outcome not observed")
	}
	return result, nil
}

func warn(event *v1.Event, field, reason string) {
	event.Warnings = append(event.Warnings, &v1.DataWarning{Field: field, Reason: reason})
}

func copyOptional[T any](value *T) *T {
	if value == nil {
		return nil
	}
	result := *value
	return &result
}
