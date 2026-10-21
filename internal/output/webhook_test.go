// SPDX-License-Identifier: Apache-2.0

package output

import (
	"context"
	"errors"
	"io"
	"log"
	"net/http"
	"net/http/httptest"
	"strings"
	"sync/atomic"
	"testing"
	"time"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
	"google.golang.org/protobuf/encoding/protojson"
	"google.golang.org/protobuf/proto"
)

func webhookSpec(url string) config.Destination {
	return config.Destination{
		Name: "webhook", Type: "webhook", Format: "json", URL: url,
		Timeout: config.Duration(time.Second), AllowHTTP: true,
	}
}

func TestWebhookJSONHeadersAndRetryCounters(t *testing.T) {
	type received struct {
		method, contentType, authorization, recordID string
		body                                         []byte
		err                                          error
	}
	requests := make(chan received, 2)
	var count atomic.Int32
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		body, err := io.ReadAll(io.LimitReader(request.Body, 1<<20))
		requests <- received{
			request.Method, request.Header.Get("Content-Type"), request.Header.Get("Authorization"),
			request.Header.Get("X-BuoyBSD-Record-ID"), body, err,
		}
		if count.Add(1) == 1 {
			writer.WriteHeader(http.StatusServiceUnavailable)
			return
		}
		writer.WriteHeader(http.StatusNoContent)
	}))
	defer server.Close()
	spec := webhookSpec(server.URL)
	spec.Retries = 1
	spec.Headers = map[string]string{"Authorization": "fixture-token"}
	destination, err := NewWebhook(spec)
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := destination.Close(); err != nil {
			t.Error(err)
		}
	}()
	if err := destination.WriteEvent(context.Background(), sampleEvent()); err != nil {
		t.Fatal(err)
	}
	if destination.Stats() != (Stats{Attempts: 2, Errors: 1}) {
		t.Fatalf("retry failures were not observable: %+v", destination.Stats())
	}
	for range 2 {
		result := <-requests
		if result.err != nil || result.method != "POST" || result.contentType != "application/json" ||
			result.authorization != "fixture-token" || result.recordID != sampleEvent().RecordId {
			t.Fatalf("incorrect request metadata: %+v", result)
		}
		var decoded v1.Event
		if err := protojson.Unmarshal(result.body, &decoded); err != nil {
			t.Fatal(err)
		}
		if !proto.Equal(&decoded, sampleEvent()) {
			t.Fatal("webhook did not retain event semantics")
		}
	}
}

func TestWebhookRetryBudgetAndStatusClassification(t *testing.T) {
	for _, status := range []int{http.StatusBadRequest, http.StatusTooManyRequests, http.StatusInternalServerError} {
		t.Run(http.StatusText(status), func(t *testing.T) {
			var requests atomic.Int32
			server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
				requests.Add(1)
				writer.WriteHeader(status)
			}))
			defer server.Close()
			spec := webhookSpec(server.URL)
			spec.Retries = 2
			destination, err := NewWebhook(spec)
			if err != nil {
				t.Fatal(err)
			}
			defer func() {
				if err := destination.Close(); err != nil {
					t.Error(err)
				}
			}()
			if err := destination.WriteEvent(context.Background(), sampleEvent()); err == nil {
				t.Fatal("failed delivery was hidden")
			}
			expected := uint64(3)
			if status == http.StatusBadRequest {
				expected = 1
			}
			if uint64(requests.Load()) != expected || destination.Stats() != (Stats{Attempts: expected, Errors: expected}) {
				t.Fatalf("wrong retry threshold: requests=%d stats=%+v", requests.Load(), destination.Stats())
			}
		})
	}
}

func TestWebhookDoesNotFollowRedirectsOrEchoSecrets(t *testing.T) {
	var leaked atomic.Bool
	target := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		leaked.Store(true)
		writer.WriteHeader(http.StatusNoContent)
	}))
	defer target.Close()
	redirect := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		http.Redirect(writer, request, target.URL+"/fixture-secret", http.StatusTemporaryRedirect)
	}))
	defer redirect.Close()
	spec := webhookSpec(redirect.URL + "/fixture-secret?token=fixture-token")
	spec.Headers = map[string]string{"Authorization": "fixture-authorization"}
	destination, err := NewWebhook(spec)
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := destination.Close(); err != nil {
			t.Error(err)
		}
	}()
	err = destination.WriteEvent(context.Background(), sampleEvent())
	if err == nil || leaked.Load() || strings.Contains(err.Error(), "fixture-") || strings.Contains(err.Error(), redirect.URL) {
		t.Fatalf("redirect followed or endpoint credentials echoed: %v", err)
	}
}

func TestWebhookRequiresTrustedTLS(t *testing.T) {
	server := httptest.NewUnstartedServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		writer.WriteHeader(http.StatusNoContent)
	}))
	server.Config.ErrorLog = log.New(io.Discard, "", 0)
	server.StartTLS()
	defer server.Close()
	destination, err := NewWebhook(webhookSpec(server.URL + "/fixture-secret"))
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := destination.Close(); err != nil {
			t.Error(err)
		}
	}()
	err = destination.WriteEvent(context.Background(), sampleEvent())
	if err == nil || strings.Contains(err.Error(), server.URL) || strings.Contains(err.Error(), "fixture-secret") {
		t.Fatalf("untrusted TLS accepted or URL echoed: %v", err)
	}
	destination.client.Transport = server.Client().Transport
	if err := destination.WriteEvent(context.Background(), sampleEvent()); err != nil {
		t.Fatalf("trusted test CA must succeed: %v", err)
	}
}

func TestWebhookTimeoutAndCancelledBackoff(t *testing.T) {
	slow := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		select {
		case <-request.Context().Done():
		case <-time.After(time.Second):
		}
	}))
	defer slow.Close()
	spec := webhookSpec(slow.URL)
	spec.Timeout = config.Duration(5 * time.Millisecond)
	destination, err := NewWebhook(spec)
	if err != nil {
		t.Fatal(err)
	}
	start := time.Now()
	if err := destination.WriteEvent(context.Background(), sampleEvent()); err == nil || time.Since(start) > time.Second {
		t.Fatal("request timeout was not bounded")
	}
	if err := destination.Close(); err != nil {
		t.Fatal(err)
	}

	attempted := make(chan struct{}, 1)
	server := httptest.NewServer(http.HandlerFunc(func(writer http.ResponseWriter, request *http.Request) {
		attempted <- struct{}{}
		writer.WriteHeader(http.StatusServiceUnavailable)
	}))
	defer server.Close()
	spec = webhookSpec(server.URL)
	spec.Retries = 5
	destination, err = NewWebhook(spec)
	if err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := destination.Close(); err != nil {
			t.Error(err)
		}
	}()
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan error, 1)
	go func() { done <- destination.WriteEvent(ctx, sampleEvent()) }()
	select {
	case <-attempted:
	case <-time.After(time.Second):
		t.Fatal("webhook was not attempted")
	}
	cancel()
	select {
	case err := <-done:
		if !errors.Is(err, context.Canceled) {
			t.Fatal(err)
		}
	case <-time.After(time.Second):
		t.Fatal("cancellation did not stop webhook retry")
	}
	if destination.Stats().Attempts != 1 {
		t.Fatal("cancelled retry budget continued sending")
	}
}
