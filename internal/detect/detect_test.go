// SPDX-License-Identifier: Apache-2.0

package detect

import (
	"testing"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
)

func ptr[T any](value T) *T { return &value }

func TestShellDetectionRetainsSourceAndDoesNotOverclaim(t *testing.T) {
	source := &v1.Event{
		Id: v1.EventId_EVENT_ID_PROCESS_EXEC, Name: event.ProcessExec, RecordId: "sha256:synthetic",
		Process: &v1.Process{ExecutablePath: ptr("/bin/sh")},
		Data:    &v1.Event_ProcessExec{ProcessExec: &v1.ProcessExec{Success: ptr(true)}},
	}
	detection, err := (ShellExec{}).Evaluate(source)
	if err != nil || detection == nil {
		t.Fatalf("missing detection: %v", err)
	}
	if detection.Id != "BUOY-0001" || detection.Severity != "informational" ||
		detection.DetectedFrom.RecordId != source.RecordId || detection.DetectedFrom.Id != source.Id {
		t.Fatalf("incorrect metadata: %+v", detection)
	}
	definition := (ShellExec{}).Definition()
	if len(definition.RequiredFields) != 2 || len(definition.EventSelectors) != 1 || definition.Description == "" {
		t.Fatal("detector contract is incomplete")
	}
}

func TestShellDetectionDoesNotInventMatches(t *testing.T) {
	for name, source := range map[string]*v1.Event{
		"no process": {Name: event.ProcessExec},
		"no path":    {Name: event.ProcessExec, Process: &v1.Process{}},
		"no payload": {Name: event.ProcessExec, Process: &v1.Process{ExecutablePath: ptr("/bin/sh")}},
		"no outcome": {Name: event.ProcessExec, Process: &v1.Process{ExecutablePath: ptr("/bin/sh")},
			Data: &v1.Event_ProcessExec{ProcessExec: &v1.ProcessExec{}}},
		"failed exec": {Name: event.ProcessExec, Process: &v1.Process{ExecutablePath: ptr("/bin/sh")},
			Data: &v1.Event_ProcessExec{ProcessExec: &v1.ProcessExec{Success: ptr(false)}}},
		"not shell": {Name: event.ProcessExec, Process: &v1.Process{ExecutablePath: ptr("/usr/bin/true")},
			Data: &v1.Event_ProcessExec{ProcessExec: &v1.ProcessExec{Success: ptr(true)}}},
		"other event": {Name: "process_exit"},
	} {
		t.Run(name, func(t *testing.T) {
			detection, err := (ShellExec{}).Evaluate(source)
			if err != nil || detection != nil {
				t.Fatalf("unexpected detection: %v %v", detection, err)
			}
		})
	}
	if _, err := (ShellExec{}).Evaluate(nil); err == nil {
		t.Fatal("nil input accepted")
	}
}
