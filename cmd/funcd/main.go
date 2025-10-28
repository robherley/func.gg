package main

import (
	"context"
	"log/slog"
	"net"
	"net/http"
	"net/http/httputil"
	"net/url"
	"os"
	"os/signal"
	"sync"
	"syscall"
	"time"

	"github.com/robherley/func.gg/internal/rt"
)

const (
	port       = "8080"
	socketPath = "/tmp/funcd.sock"
)

var (
	buntime *rt.Bun
	mu      = &sync.RWMutex{}
)

func main() {
	ctx, stop := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGINT, syscall.SIGTERM)
	defer stop()

	if err := run(ctx); err != nil {
		slog.Error("fatal error", "error", err)
		os.Exit(1)
	}
}

func run(ctx context.Context) error {
	cleanupSocket(socketPath)
	defer cleanupSocket(socketPath)
	defer cleanupBuntime()

	target := &url.URL{
		Scheme: "http",
		Host:   "unix",
	}

	transport := &http.Transport{
		DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
			return net.Dial("unix", socketPath)
		},
		MaxIdleConns:        100,
		IdleConnTimeout:     90 * time.Second,
		TLSHandshakeTimeout: 10 * time.Second,
	}

	proxy := httputil.ReverseProxy{
		Transport: transport,
	}

	originalDirector := httputil.NewSingleHostReverseProxy(target).Director
	proxy.Director = func(req *http.Request) {
		originalDirector(req)
		req.URL.Host = "unix"
		req.URL.Scheme = "http"
		slog.Info("proxying", "method", req.Method, "path", req.URL.Path)
	}

	proxy.ErrorHandler = func(w http.ResponseWriter, r *http.Request, err error) {
		slog.Error("proxy failed", "error", err)
		http.Error(w, "Bad Gateway", http.StatusBadGateway)
	}

	mux := http.NewServeMux()
	mux.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
		if _, err := getOrCreateBuntime(ctx); err != nil {
			slog.Error("failed to start bun runtime", "error", err)
			http.Error(w, "internal server error", http.StatusInternalServerError)
			return
		}

		proxy.ServeHTTP(w, r)
	})

	slog.Info("starting reverse proxy", "port", port)
	slog.Info("forwarding requests to Unix socket", "socket", socketPath)

	server := &http.Server{
		Addr:    ":" + port,
		Handler: mux,
	}

	go server.ListenAndServe()
	<-ctx.Done()
	slog.Info("shutting down server")
	return server.Shutdown(ctx)
}

func waitForSocket(path string, timeout time.Duration) error {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	ticker := time.NewTicker(1 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-ticker.C:
			if _, err := os.Stat(path); err == nil {
				return nil
			}
		}
	}
}

func cleanupSocket(path string) {
	if _, err := os.Stat(path); err == nil {
		if err := os.Remove(path); err != nil {
			slog.Error("failed to remove socket file", "error", err)
		} else {
			slog.Info("removed existing socket file", "path", path)
		}
	}
}

func getOrCreateBuntime(ctx context.Context) (*rt.Bun, error) {
	mu.Lock()
	defer mu.Unlock()

	if buntime == nil {
		buntime = rt.NewBuntime(socketPath)
		if err := buntime.Start(ctx); err != nil {
			return nil, err
		}
		return nil, waitForSocket(socketPath, 1*time.Second)
	}

	return buntime, nil
}

func cleanupBuntime() {
	mu.Lock()
	defer mu.Unlock()

	if buntime != nil {
		buntime.Kill()
		buntime = nil
	}
}
