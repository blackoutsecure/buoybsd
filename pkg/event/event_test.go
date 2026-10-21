// SPDX-License-Identifier: Apache-2.0

package event

import "testing"

func TestEventCatalogueAndNamespace(t *testing.T) {
	definitions := Definitions()
	if len(definitions) != 1 || definitions[0].Name != ProcessExec ||
		definitions[0].ID < 1000000 || definitions[0].ID > 1999999 || !Known(ProcessExec) ||
		Known("sched_process_exec") || Known("process_exit") {
		t.Fatal("event catalogue exposes unsupported events or Tracee IDs")
	}
	definitions[0].Name = "changed"
	if Definitions()[0].Name != ProcessExec {
		t.Fatal("catalogue exposes mutable global definitions")
	}
}
