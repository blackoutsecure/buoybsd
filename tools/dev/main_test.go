// SPDX-License-Identifier: Apache-2.0

package main

import (
	"slices"
	"strings"
	"testing"
)

func TestEnvironmentOverrideDoesNotDuplicateKeys(t *testing.T) {
	got := setEnv([]string{"Path=keep", "GOOS=windows", "goos=other", "CGO_ENABLED=1"}, "GOOS", "freebsd")
	if !slices.Contains(got, "Path=keep") || !slices.Contains(got, "GOOS=freebsd") {
		t.Fatal(got)
	}
	count := 0
	for _, item := range got {
		name, _, _ := strings.Cut(item, "=")
		if strings.EqualFold(name, "GOOS") {
			count++
		}
	}
	if count != 1 {
		t.Fatal("GOOS was not deterministically overridden")
	}
}
