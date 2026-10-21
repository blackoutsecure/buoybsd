// SPDX-License-Identifier: Apache-2.0

package policy

import (
	"errors"
	"path"
	"slices"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
)

type Set struct {
	policies []config.Policy
}

func Compile(policies []config.Policy) (*Set, error) {
	if err := config.ValidatePolicies(policies); err != nil {
		return nil, err
	}
	return &Set{policies: policies}, nil
}

func (s *Set) Match(event *v1.Event) (bool, []string, error) {
	if event == nil {
		return false, nil, errors.New("policy evaluation requires an event")
	}
	if len(s.policies) == 0 {
		return true, nil, nil
	}
	var matched []string
	for _, policy := range s.policies {
		match, err := matches(policy, event)
		if err != nil {
			return false, nil, err
		}
		if match {
			matched = append(matched, policy.Name)
		}
	}
	return len(matched) != 0, matched, nil
}

func matches(policy config.Policy, event *v1.Event) (bool, error) {
	if len(policy.EventNames) != 0 && !slices.Contains(policy.EventNames, event.Name) {
		return false, nil
	}
	if len(policy.Collectors) != 0 && !slices.Contains(policy.Collectors, event.GetSource().GetCollector()) {
		return false, nil
	}
	process := event.Process
	if process == nil {
		if len(policy.Executables)+len(policy.ProcessNames)+len(policy.UIDs)+len(policy.GIDs) != 0 {
			return false, nil
		}
	} else {
		if !numeric(policy.UIDs, process.Uid) || !numeric(policy.GIDs, process.Gid) {
			return false, nil
		}
		executable, err := glob(policy.Executables, process.ExecutablePath)
		if err != nil || !executable {
			return false, err
		}
		command, err := glob(policy.ProcessNames, process.Command)
		if err != nil || !command {
			return false, err
		}
	}
	var jailID *uint32
	if event.Jail != nil {
		jailID = event.Jail.Id
	}
	return numeric(policy.JailIDs, jailID), nil
}

func numeric(allowed []uint32, observed *uint32) bool {
	return len(allowed) == 0 || observed != nil && slices.Contains(allowed, *observed)
}

func glob(patterns []string, observed *string) (bool, error) {
	if len(patterns) == 0 {
		return true, nil
	}
	if observed == nil {
		return false, nil
	}
	for _, pattern := range patterns {
		matched, err := path.Match(pattern, *observed)
		if err != nil {
			return false, errors.New("invalid compiled policy glob")
		}
		if matched {
			return true, nil
		}
	}
	return false, nil
}
