// SPDX-License-Identifier: Apache-2.0

package detect

import (
	"errors"
	"slices"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
)

type Definition struct {
	ID             string
	Name           string
	Description    string
	EventSelectors []string
	Severity       string
	Tags           []string
	RequiredFields []string
}

type Detector interface {
	Definition() Definition
	Evaluate(*v1.Event) (*v1.Detection, error)
}

type ShellExec struct{}

func (ShellExec) Definition() Definition {
	return Definition{
		ID:             "BUOY-0001",
		Name:           "Shell execution",
		Description:    "A known shell executable was observed executing successfully; not proof of malicious activity.",
		EventSelectors: []string{event.ProcessExec},
		Severity:       "informational",
		Tags:           []string{"execution", "shell"},
		RequiredFields: []string{"process.executable_path", "process_exec.success"},
	}
}

func (s ShellExec) Evaluate(source *v1.Event) (*v1.Detection, error) {
	if source == nil {
		return nil, errors.New("detector evaluation requires an event")
	}
	if source.Name != event.ProcessExec || source.Process == nil || source.Process.ExecutablePath == nil {
		return nil, nil
	}
	execution := source.GetProcessExec()
	if execution == nil || execution.Success == nil || !execution.GetSuccess() {
		return nil, nil
	}
	shells := []string{"/bin/sh", "/bin/csh", "/bin/tcsh", "/usr/local/bin/bash", "/usr/local/bin/zsh"}
	if !slices.Contains(shells, *source.Process.ExecutablePath) {
		return nil, nil
	}
	definition := s.Definition()
	return &v1.Detection{
		Id: definition.ID, Name: definition.Name, Description: definition.Description,
		Severity: definition.Severity, Tags: definition.Tags,
		DetectedFrom: &v1.EventReference{Id: source.Id, Name: source.Name, RecordId: source.RecordId},
	}, nil
}
