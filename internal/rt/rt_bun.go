package rt

import (
	"context"
	"os"
	"os/exec"
)

type Bun struct {
	socketPath string
	cmd        *exec.Cmd
}

func NewBuntime(socketPath string) *Bun {
	return &Bun{
		socketPath: socketPath,
	}
}

func (b *Bun) Command() (string, []string) {
	return "bun", []string{"run", "/Users/robherley/dev/func.gg/js/serve.js"}
}

func (b *Bun) Start(ctx context.Context) error {
	binary, args := b.Command()
	b.cmd = exec.CommandContext(ctx, binary, args...)
	b.cmd.Env = append(b.cmd.Env, "FUNCD_SOCKET_PATH="+b.socketPath)

	b.cmd.Stdout = os.Stdout
	b.cmd.Stderr = os.Stderr
	// b.cmd.Stdin = os.Stdin

	return b.cmd.Start()
}

func (b *Bun) Wait() error {
	if b.cmd != nil {
		return b.cmd.Wait()
	}
	return nil
}

func (b *Bun) Signal(sig os.Signal) error {
	if b.cmd != nil && b.cmd.Process != nil {
		return b.cmd.Process.Signal(sig)
	}
	return nil
}

func (b *Bun) Kill() error {
	if b.cmd != nil && b.cmd.Process != nil {
		return b.cmd.Process.Kill()
	}
	return nil
}
