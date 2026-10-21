// SPDX-License-Identifier: Apache-2.0

package output

import (
	"bytes"
	"context"
	"crypto/tls"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"time"

	v1 "github.com/blackoutsecure/int-buoybsd/api/v1beta1"
	"github.com/blackoutsecure/int-buoybsd/pkg/config"
)

type Webhook struct {
	counters
	spec   config.Destination
	client *http.Client
}

func NewWebhook(spec config.Destination) (*Webhook, error) {
	if spec.Type != "webhook" {
		return nil, errors.New("webhook output requires type webhook")
	}
	if err := spec.Validate(); err != nil {
		return nil, err
	}
	timeout := time.Duration(spec.Timeout)
	transport := &http.Transport{
		Proxy:                 http.ProxyFromEnvironment,
		DialContext:           (&net.Dialer{Timeout: timeout, KeepAlive: 30 * time.Second}).DialContext,
		TLSClientConfig:       &tls.Config{MinVersion: tls.VersionTLS12},
		TLSHandshakeTimeout:   timeout,
		ResponseHeaderTimeout: timeout,
		IdleConnTimeout:       30 * time.Second,
		MaxIdleConns:          1,
		MaxIdleConnsPerHost:   1,
		ForceAttemptHTTP2:     true,
	}
	return &Webhook{
		spec: spec,
		client: &http.Client{
			Timeout: timeout, Transport: transport,
			CheckRedirect: func(*http.Request, []*http.Request) error { return http.ErrUseLastResponse },
		},
	}, nil
}

func (w *Webhook) Name() string { return w.spec.Name }

func (w *Webhook) Close() error {
	w.client.CloseIdleConnections()
	return nil
}

func (w *Webhook) WriteEvent(ctx context.Context, event *v1.Event) error {
	body, err := Marshal(event)
	if err != nil {
		w.attempts.Add(1)
		return w.complete(err)
	}
	for attempt := 0; attempt <= w.spec.Retries; attempt++ {
		if err := ctx.Err(); err != nil {
			return err
		}
		w.attempts.Add(1)
		retry, err := w.deliver(ctx, event.RecordId, body)
		if err == nil {
			return nil
		}
		w.failures.Add(1)
		if !retry || attempt == w.spec.Retries {
			return err
		}
		delay := time.NewTimer(100 * time.Millisecond * time.Duration(1<<attempt))
		select {
		case <-delay.C:
		case <-ctx.Done():
			delay.Stop()
			return ctx.Err()
		}
	}
	return errors.New("webhook delivery exhausted its retry budget")
}

func (w *Webhook) deliver(ctx context.Context, recordID string, body []byte) (bool, error) {
	request, err := http.NewRequestWithContext(ctx, http.MethodPost, w.spec.URL, bytes.NewReader(body))
	if err != nil {
		return false, errors.New("cannot construct webhook request")
	}
	request.Header.Set("Content-Type", "application/json")
	request.Header.Set("X-BuoyBSD-Record-ID", recordID)
	for name, value := range w.spec.Headers {
		request.Header.Set(name, value)
	}
	response, err := w.client.Do(request)
	if err != nil {
		if ctx.Err() != nil {
			return false, ctx.Err()
		}
		return true, errors.New("webhook transport failed: check connectivity, TLS trust, and timeout")
	}
	_, readErr := io.Copy(io.Discard, io.LimitReader(response.Body, 64<<10))
	closeErr := response.Body.Close()
	if readErr != nil || closeErr != nil {
		return true, errors.New("webhook response read or close failed")
	}
	if response.StatusCode >= 200 && response.StatusCode < 300 {
		return false, nil
	}
	retry := response.StatusCode == http.StatusTooManyRequests || response.StatusCode >= 500
	return retry, fmt.Errorf("webhook returned HTTP %d", response.StatusCode)
}
