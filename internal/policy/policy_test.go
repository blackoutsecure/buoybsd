// SPDX-License-Identifier: Apache-2.0

package policy

import (
	"testing"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
	"github.com/blackoutsecure/int-buoybsd/pkg/event"
)

func ptr[T any](value T) *T { return &value }

func TestPolicySelectorsAndMissingValues(t *testing.T) {
	input := &v1.Event{
		Name: event.ProcessExec, Source: &v1.Source{Collector: "replay"},
		Process: &v1.Process{
			Uid: ptr(uint32(0)), Euid: ptr(uint32(1000)), Gid: ptr(uint32(0)),
			ExecutablePath: ptr("/usr/local/bin/bash"), Command: ptr("bash"),
		},
		Jail: &v1.Jail{Id: ptr(uint32(0))},
	}
	tests := []struct {
		name   string
		policy config.Policy
		match  bool
	}{
		{"empty selectors", config.Policy{Name: "all"}, true},
		{"event", config.Policy{Name: "event", EventNames: []string{"process_exec"}}, true},
		{"collector", config.Policy{Name: "collector", Collectors: []string{"replay"}}, true},
		{"UID zero", config.Policy{Name: "root", UIDs: []uint32{0}}, true},
		{"not effective UID", config.Policy{Name: "real", UIDs: []uint32{1000}}, false},
		{"GID zero", config.Policy{Name: "group", GIDs: []uint32{0}}, true},
		{"jail zero", config.Policy{Name: "jail", JailIDs: []uint32{0}}, true},
		{"exact executable", config.Policy{Name: "path", Executables: []string{"/usr/local/bin/bash"}}, true},
		{"glob executable", config.Policy{Name: "path", Executables: []string{"/usr/local/bin/*"}}, true},
		{"glob cannot cross slash", config.Policy{Name: "path", Executables: []string{"/usr/*"}}, false},
		{"command", config.Policy{Name: "command", ProcessNames: []string{"ba*"}}, true},
		{"AND categories", config.Policy{Name: "and", UIDs: []uint32{0}, GIDs: []uint32{1000}}, false},
		{"OR values", config.Policy{Name: "or", UIDs: []uint32{1000, 0}}, true},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			set, err := Compile([]config.Policy{test.policy})
			if err != nil {
				t.Fatal(err)
			}
			match, names, err := set.Match(input)
			if err != nil || match != test.match || len(names) != map[bool]int{true: 1, false: 0}[test.match] {
				t.Fatalf("match=%v names=%v err=%v", match, names, err)
			}
		})
	}
	for _, spec := range []config.Policy{
		{Name: "UID", UIDs: []uint32{0}},
		{Name: "GID", GIDs: []uint32{0}},
		{Name: "jail", JailIDs: []uint32{0}},
		{Name: "path", Executables: []string{"*"}},
		{Name: "command", ProcessNames: []string{"*"}},
	} {
		set, err := Compile([]config.Policy{spec})
		if err != nil {
			t.Fatal(err)
		}
		for _, missing := range []*v1.Event{{Name: event.ProcessExec}, {Name: event.ProcessExec, Process: &v1.Process{}, Jail: &v1.Jail{}}} {
			match, _, err := set.Match(missing)
			if err != nil || match {
				t.Fatalf("missing values matched %s: %v", spec.Name, err)
			}
		}
	}
}

func TestPolicyUnionDefaultsAndValidation(t *testing.T) {
	set, err := Compile([]config.Policy{{Name: "one"}, {Name: "two"}})
	if err != nil {
		t.Fatal(err)
	}
	match, names, err := set.Match(&v1.Event{Name: event.ProcessExec})
	if err != nil || !match || len(names) != 2 || names[0] != "one" || names[1] != "two" {
		t.Fatalf("policy union failed: %v %v %v", match, names, err)
	}
	set, err = Compile(nil)
	if err != nil {
		t.Fatal(err)
	}
	if match, names, err := set.Match(&v1.Event{}); err != nil || !match || len(names) != 0 {
		t.Fatal("empty policy set must retain all")
	}
	if _, _, err := set.Match(nil); err == nil {
		t.Fatal("nil event accepted")
	}
	for _, specs := range [][]config.Policy{
		{{Name: "bad", Executables: []string{"["}}},
		{{Name: "bad", EventNames: []string{"process_exit"}}},
		{{Name: "bad", Collectors: []string{"audit"}}},
		{{Name: "same"}, {Name: "same"}},
	} {
		if _, err := Compile(specs); err == nil {
			t.Fatalf("invalid policy set accepted: %+v", specs)
		}
	}
}
