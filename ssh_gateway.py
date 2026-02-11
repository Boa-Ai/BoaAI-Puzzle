#!/usr/bin/env python3
"""Credentialless SSH gateway for the BoaAI puzzle app.

This accepts SSH connections with no authentication and launches one puzzle
process per client (concurrent sessions supported).
"""

from __future__ import annotations

import argparse
import asyncio
import errno
import os
import pty
import struct
import termios
from pathlib import Path
from typing import Awaitable, Callable

import asyncssh

DEFAULT_HOST = "0.0.0.0"
DEFAULT_PORT = 1337
DEFAULT_BACKLOG = 512


def resolve_binary(path_hint: str | None) -> Path:
    if path_hint:
        candidate = Path(path_hint).expanduser().resolve()
        if not candidate.exists():
            raise FileNotFoundError(f"Puzzle binary not found: {candidate}")
        return candidate

    root = Path(__file__).resolve().parent
    candidates = [
        root / "target" / "release" / "ssh_store",
        root / "target" / "debug" / "ssh_store",
    ]
    for candidate in candidates:
        if candidate.exists():
            return candidate

    raise FileNotFoundError(
        "Could not find puzzle binary. Build it first with `cargo build --release`."
    )


def ensure_host_key(path: Path) -> Path:
    path = path.expanduser().resolve()
    path.parent.mkdir(parents=True, exist_ok=True)

    if path.exists():
        return path

    key = asyncssh.generate_private_key("ssh-ed25519")
    private_key = key.export_private_key()
    if isinstance(private_key, bytes):
        path.write_bytes(private_key)
    else:
        path.write_text(private_key)
    os.chmod(path, 0o600)
    return path


def set_pty_size(fd: int, cols: int, rows: int) -> None:
    cols = max(cols, 20)
    rows = max(rows, 10)
    winsize = struct.pack("HHHH", rows, cols, 0, 0)
    termios.tcsetwinsize(fd, (rows, cols))
    try:
        import fcntl  # imported lazily, Linux-only

        fcntl.ioctl(fd, termios.TIOCSWINSZ, winsize)
    except Exception:
        pass


class NoAuthSSHServer(asyncssh.SSHServer):
    def begin_auth(self, _username: str) -> bool:
        return False


def _write_all(fd: int, data: bytes) -> None:
    view = memoryview(data)
    while view:
        written = os.write(fd, view)
        if written <= 0:
            raise OSError("short write to PTY")
        view = view[written:]


def make_process_handler(binary: Path) -> Callable[[asyncssh.SSHServerProcess], Awaitable[None]]:
    async def handle(process: asyncssh.SSHServerProcess) -> None:
        master_fd, slave_fd = pty.openpty()
        child = None
        ssh_to_pty_task = None
        pty_to_ssh_task = None
        child_wait_task = None
        client_wait_task = None

        try:
            cols, rows, _, _ = process.get_terminal_size()
            if cols == 0 or rows == 0:
                cols, rows = 120, 40
            set_pty_size(slave_fd, cols, rows)

            child = await asyncio.create_subprocess_exec(
                str(binary),
                stdin=slave_fd,
                stdout=slave_fd,
                stderr=slave_fd,
                start_new_session=True,
            )
            os.close(slave_fd)
            slave_fd = None

            async def ssh_to_pty() -> None:
                try:
                    while True:
                        data = await process.stdin.read(8192)
                        if not data:
                            break
                        await asyncio.to_thread(_write_all, master_fd, data)
                except (asyncssh.Error, OSError, BrokenPipeError):
                    pass

            async def pty_to_ssh() -> None:
                while True:
                    try:
                        data = await asyncio.to_thread(os.read, master_fd, 8192)
                    except OSError as exc:
                        # PTY returns EIO when slave side closes (normal on child exit).
                        if exc.errno == errno.EIO:
                            break
                        raise

                    if not data:
                        break

                    try:
                        process.stdout.write(data)
                        await process.stdout.drain()
                    except asyncssh.Error:
                        break

            ssh_to_pty_task = asyncio.create_task(ssh_to_pty())
            pty_to_ssh_task = asyncio.create_task(pty_to_ssh())
            child_wait_task = asyncio.create_task(child.wait())
            client_wait_task = asyncio.create_task(process.wait_closed())

            done, pending = await asyncio.wait(
                {child_wait_task, client_wait_task, ssh_to_pty_task, pty_to_ssh_task},
                return_when=asyncio.FIRST_COMPLETED,
            )

            if child_wait_task in done and not process.is_closing():
                process.exit(child.returncode or 0)

            if client_wait_task in done and child.returncode is None:
                child.terminate()
                try:
                    await asyncio.wait_for(child.wait(), timeout=3)
                except asyncio.TimeoutError:
                    child.kill()
                    await child.wait()

            for task in pending:
                task.cancel()

            await asyncio.gather(
                ssh_to_pty_task,
                pty_to_ssh_task,
                child_wait_task,
                client_wait_task,
                return_exceptions=True,
            )
        except (asyncssh.Error, OSError, BrokenPipeError):
            # Connection churn is expected under internet traffic; keep handler quiet.
            pass
        finally:
            if child is not None and child.returncode is None:
                child.terminate()
                try:
                    await asyncio.wait_for(child.wait(), timeout=3)
                except asyncio.TimeoutError:
                    child.kill()
                    await child.wait()
            for task in (ssh_to_pty_task, pty_to_ssh_task, child_wait_task, client_wait_task):
                if task is not None and not task.done():
                    task.cancel()
            try:
                os.close(master_fd)
            except OSError:
                pass
            if slave_fd is not None:
                try:
                    os.close(slave_fd)
                except OSError:
                    pass

    return handle


async def run_server(host: str, port: int, backlog: int, host_key: Path, binary: Path) -> None:
    server = await asyncssh.listen(
        host,
        port,
        server_factory=NoAuthSSHServer,
        process_factory=make_process_handler(binary),
        server_host_keys=[str(host_key)],
        backlog=backlog,
        encoding=None,
        reuse_address=True,
    )

    print(f"SSH gateway listening on {host}:{port}", flush=True)
    print(f"Puzzle binary: {binary}", flush=True)
    print("Authentication: DISABLED (publicly accessible)", flush=True)

    await server.wait_closed()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run BoaAI credentialless SSH gateway on port 1337."
    )
    parser.add_argument("--host", default=DEFAULT_HOST, help="Bind host (default: 0.0.0.0)")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="Bind port (default: 1337)")
    parser.add_argument(
        "--backlog", type=int, default=DEFAULT_BACKLOG, help="Socket backlog (default: 512)"
    )
    parser.add_argument(
        "--host-key",
        default=".ssh/boaai_ssh_host_ed25519",
        help="Path to SSH host key file",
    )
    parser.add_argument(
        "--binary",
        default=os.getenv("BOAAI_PUZZLE_BIN"),
        help="Path to puzzle binary (default: target/release or target/debug ssh_store)",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()

    try:
        binary = resolve_binary(args.binary)
        host_key = ensure_host_key(Path(args.host_key))
    except Exception as exc:
        print(f"Startup error: {exc}")
        return 1

    try:
        asyncio.run(run_server(args.host, args.port, args.backlog, host_key, binary))
    except KeyboardInterrupt:
        pass
    except Exception as exc:
        print(f"Gateway error: {exc}")
        return 1

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
