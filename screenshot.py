"""Screenshots of VS Code with the extension under development, on any desktop (Wayland too).

It starts its own VS Code (separate profile, the extension from editors/vscode, a DevTools port)
and drives it over the Chrome DevTools Protocol: keys, text, screenshots. Standard library only.

Usage:
  uv run screenshot.py launch FOLDER [--ext DIR] [--port N] [--size WxH]
  uv run screenshot.py keys KEY|text:TEXT|wait:SECONDS ... [--port N]
  uv run screenshot.py shot OUT.png [--port N]
  uv run screenshot.py close [--port N]

Keys are names like F1, Enter, Escape, Tab, ArrowDown. For example, open the History panel:
  uv run screenshot.py keys F1 "text:sundgren: Show History" Enter wait:1 "text:main" Enter
"""

from __future__ import annotations

import argparse
import base64
import contextlib
import json
import os
import socket
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent
PORT = 9333
# Key name: (code, Windows virtual key code), as Input.dispatchKeyEvent wants them.
KEYS = {
    "Enter": ("Enter", 13),
    "Escape": ("Escape", 27),
    "Tab": ("Tab", 9),
    "Backspace": ("Backspace", 8),
    "ArrowUp": ("ArrowUp", 38),
    "ArrowDown": ("ArrowDown", 40),
    "ArrowLeft": ("ArrowLeft", 37),
    "ArrowRight": ("ArrowRight", 39),
    **{f"F{n}": (f"F{n}", 111 + n) for n in range(1, 13)},
}
# A quiet window: no welcome page, no trust prompt, no update or telemetry noise.
SETTINGS = {
    "workbench.startupEditor": "none",
    "workbench.tips.enabled": False,
    "window.restoreWindows": "none",
    "security.workspace.trust.enabled": False,
    "update.mode": "none",
    "telemetry.telemetryLevel": "off",
    "extensions.autoUpdate": False,
    "extensions.ignoreRecommendations": True,
    "git.openRepositoryInParentFolders": "never",
    "chat.disableAIFeatures": True,
}


class WebSocket:
    """Just enough of RFC 6455 for DevTools: text frames out (masked), whole messages in."""

    def __init__(self, url: str) -> None:
        rest = url.removeprefix("ws://")
        host_port, _, path = rest.partition("/")
        host, _, port = host_port.partition(":")
        self.sock = socket.create_connection((host, int(port or 80)), timeout=30)
        key = base64.b64encode(os.urandom(16)).decode()
        request = (
            f"GET /{path} HTTP/1.1\r\nHost: {host_port}\r\nUpgrade: websocket\r\n"
            f"Connection: Upgrade\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )
        self.sock.sendall(request.encode())
        head = b""
        while b"\r\n\r\n" not in head:
            chunk = self.sock.recv(1024)
            if not chunk:
                raise ConnectionError("DevTools closed the connection during the handshake")
            head += chunk
        status = head.split(b"\r\n", 1)[0]
        if b" 101 " not in status:
            raise ConnectionError(f"DevTools refused the websocket: {status.decode()}")
        self.buffer = head.split(b"\r\n\r\n", 1)[1]

    def _read(self, n: int) -> bytes:
        while len(self.buffer) < n:
            chunk = self.sock.recv(65536)
            if not chunk:
                raise ConnectionError("DevTools closed the connection")
            self.buffer += chunk
        out, self.buffer = self.buffer[:n], self.buffer[n:]
        return out

    def _frame(self, opcode: int, payload: bytes) -> None:
        mask = os.urandom(4)
        n = len(payload)
        if n < 126:
            header = struct.pack("!BB", 0x80 | opcode, 0x80 | n)
        elif n < 1 << 16:
            header = struct.pack("!BBH", 0x80 | opcode, 0x80 | 126, n)
        else:
            header = struct.pack("!BBQ", 0x80 | opcode, 0x80 | 127, n)
        masked = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
        self.sock.sendall(header + mask + masked)

    def send(self, text: str) -> None:
        self._frame(0x1, text.encode())

    def receive(self) -> str:
        message = b""
        while True:
            first, second = self._read(2)
            opcode, n = first & 0x0F, second & 0x7F
            if n == 126:
                (n,) = struct.unpack("!H", self._read(2))
            elif n == 127:
                (n,) = struct.unpack("!Q", self._read(8))
            payload = self._read(n)
            if opcode == 0x9:  # ping
                self._frame(0xA, payload)
                continue
            if opcode == 0x8:
                raise ConnectionError("DevTools closed the websocket")
            message += payload
            if first & 0x80:
                return message.decode()

    def close(self) -> None:
        self.sock.close()


class DevTools:
    """One DevTools target: call(method, params) returns the result."""

    def __init__(self, url: str) -> None:
        self.ws = WebSocket(url)
        self.next_id = 0

    def call(self, method: str, **params: Any) -> dict[str, Any]:
        self.next_id += 1
        self.ws.send(json.dumps({"id": self.next_id, "method": method, "params": params}))
        while True:
            reply: dict[str, Any] = json.loads(self.ws.receive())
            if reply.get("id") != self.next_id:
                continue  # an event
            if "error" in reply:
                raise RuntimeError(f"{method}: {reply['error']}")
            result: dict[str, Any] = reply.get("result", {})
            return result


def targets(port: int) -> list[dict[str, Any]]:
    with urllib.request.urlopen(f"http://127.0.0.1:{port}/json", timeout=5) as response:
        found: list[dict[str, Any]] = json.load(response)
    return found


def workbench(port: int) -> DevTools:
    """The VS Code window (its workbench page)."""
    for t in targets(port):
        if t.get("type") == "page" and "workbench" in str(t.get("url", "")):
            return DevTools(str(t["webSocketDebuggerUrl"]))
    raise SystemExit(f"no VS Code window on port {port}; run launch first")


def launch(folder: Path, ext: Path, port: int, size: str) -> None:
    profile = Path(tempfile.gettempdir()) / f"sundgren-screenshot-{port}"
    settings = profile / "User" / "settings.json"
    settings.parent.mkdir(parents=True, exist_ok=True)
    settings.write_text(json.dumps(SETTINGS, indent=2), "utf-8")
    width, _, height = size.partition("x")
    subprocess.Popen(
        [
            "code",
            "--new-window",
            f"--user-data-dir={profile}",
            f"--extensions-dir={profile / 'extensions'}",
            f"--extensionDevelopmentPath={ext.resolve()}",
            f"--remote-debugging-port={port}",
            f"--window-size={width},{height}",
            str(folder.resolve()),
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        start_new_session=True,
    )
    deadline = time.monotonic() + 60
    while time.monotonic() < deadline:
        try:
            tools = workbench(port)
            ready = tools.call(
                "Runtime.evaluate",
                expression="document.querySelector('.monaco-workbench .part.editor') !== null",
                returnByValue=True,
            )
            tools.ws.close()
            if ready.get("result", {}).get("value") is True:
                time.sleep(2)  # extensions activate after the workbench shows
                print(f"VS Code is up on port {port} (profile {profile})")
                return
        except (OSError, SystemExit):
            pass
        time.sleep(0.5)
    raise SystemExit("VS Code didn't open a window within 60 s")


def keys(port: int, items: list[str]) -> None:
    tools = workbench(port)
    for item in items:
        if item.startswith("text:"):
            tools.call("Input.insertText", text=item.removeprefix("text:"))
        elif item.startswith("wait:"):
            time.sleep(float(item.removeprefix("wait:")))
        elif item in KEYS:
            code, vk = KEYS[item]
            for kind in ("rawKeyDown", "keyUp"):
                tools.call(
                    "Input.dispatchKeyEvent",
                    type=kind,
                    key=item,
                    code=code,
                    windowsVirtualKeyCode=vk,
                    nativeVirtualKeyCode=vk,
                )
            if item == "Enter":
                tools.call("Input.dispatchKeyEvent", type="char", text="\r")
        else:
            raise SystemExit(f"unknown key {item!r}; use one of {', '.join(KEYS)}, text:, wait:")
        time.sleep(0.2)
    tools.ws.close()


def shot(port: int, out: Path) -> None:
    tools = workbench(port)
    data = tools.call("Page.captureScreenshot", format="png")["data"]
    tools.ws.close()
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(base64.b64decode(data))
    print(out)


def close(port: int) -> None:
    with urllib.request.urlopen(f"http://127.0.0.1:{port}/json/version", timeout=5) as response:
        url = str(json.load(response)["webSocketDebuggerUrl"])
    # It may close before answering.
    with contextlib.suppress(ConnectionError):
        DevTools(url).call("Browser.close")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0] if __doc__ else None)
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--port", type=int, default=PORT, help=f"DevTools port (default {PORT})")
    commands = parser.add_subparsers(dest="command", required=True)
    p = commands.add_parser("launch", parents=[common])
    p.add_argument("folder", type=Path)
    p.add_argument("--ext", type=Path, default=ROOT / "editors" / "vscode")
    p.add_argument("--size", default="1500x950")
    p = commands.add_parser("keys", parents=[common])
    p.add_argument("items", nargs="+")
    p = commands.add_parser("shot", parents=[common])
    p.add_argument("out", type=Path)
    commands.add_parser("close", parents=[common])
    args = parser.parse_args()
    port: int = args.port
    if args.command == "launch":
        launch(args.folder, args.ext, port, args.size)
    elif args.command == "keys":
        keys(port, args.items)
    elif args.command == "shot":
        shot(port, args.out)
    else:
        close(port)


if __name__ == "__main__":
    sys.exit(main())
