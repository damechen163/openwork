import { BrowserWindow as e, app as t, ipcMain as n } from "electron";
import { spawn as r } from "node:child_process";
import i from "node:path";
import { fileURLToPath as a } from "node:url";
//#region electron/main.ts
t.commandLine.appendSwitch("no-sandbox");
var o = i.dirname(a(import.meta.url)), s = null, c = null;
function l() {
	let e = process.env.OPENWORK_BIN;
	return e && e.trim().length > 0 ? e : "openwork";
}
function u(e) {
	return new Promise((t) => {
		let n = r(l(), e, { env: { ...process.env } }), i = "", a = "";
		n.stdout.on("data", (e) => {
			i += e;
		}), n.stderr.on("data", (e) => {
			a += e;
		}), n.on("error", (e) => {
			t({
				ok: !1,
				exitCode: null,
				stdout: i,
				stderr: e.message
			});
		}), n.on("close", (e) => {
			t({
				ok: e === 0,
				exitCode: e,
				stdout: i,
				stderr: a
			});
		});
	});
}
function d() {
	s = new e({
		width: 1280,
		height: 860,
		minWidth: 960,
		minHeight: 640,
		backgroundColor: "#0f1419",
		title: "OpenWork Admin",
		webPreferences: {
			preload: i.join(o, "preload.cjs"),
			contextIsolation: !0,
			nodeIntegration: !1,
			sandbox: !1
		}
	});
	let t = process.env.VITE_DEV_SERVER_URL;
	t ? s.loadURL(t) : s.loadFile(i.join(o, "../dist/index.html"));
}
function f() {
	n.handle("openwork:version", async () => u(["--version"])), n.handle("openwork:status", async () => u(["status", "--json"])), n.handle("openwork:doctor", async () => u(["doctor", "--json"])), n.handle("openwork:installPlan", async (e, t) => {
		let n = [
			"install",
			"--dry-run",
			"--json"
		];
		return t.runtime && n.push("--runtime", t.runtime), t.version && n.push("--version", t.version), u(n);
	}), n.handle("openwork:installExecute", async (e, t) => {
		let n = [
			"install",
			"--execute",
			"--yes",
			"--json"
		];
		return t.runtime && n.push("--runtime", t.runtime), t.version && n.push("--version", t.version), u(n);
	}), n.handle("openwork:runtimeList", async () => u([
		"runtime",
		"list",
		"--json"
	])), n.handle("openwork:runtimeInfo", async (e, t) => u([
		"runtime",
		"info",
		t,
		"--json"
	])), n.handle("openwork:run", async (e, t) => {
		if (c) return {
			ok: !1,
			exitCode: null,
			stdout: "",
			stderr: "a run is already in progress"
		};
		let n = [
			"run",
			"--workspace",
			t.workspace,
			"--runtime",
			t.runtime,
			"--timeout",
			String(t.timeout),
			"--sandbox-timeout",
			String(t.sandboxTimeout),
			"--json",
			t.prompt
		];
		return new Promise((t) => {
			let i = r(l(), n, { env: { ...process.env } });
			c = i;
			let a = "", o = "";
			i.stdout.on("data", (t) => {
				let n = t.toString();
				a += n;
				for (let t of n.split("\n")) t.trim().length > 0 && e.sender.send("run:output", {
					stream: "stdout",
					line: t
				});
			}), i.stderr.on("data", (t) => {
				let n = t.toString();
				o += n;
				for (let t of n.split("\n")) t.trim().length > 0 && e.sender.send("run:output", {
					stream: "stderr",
					line: t
				});
			}), i.on("error", (n) => {
				c = null, e.sender.send("run:done", {
					ok: !1,
					exitCode: null,
					stdout: a,
					stderr: n.message
				}), t({
					ok: !1,
					exitCode: null,
					stdout: a,
					stderr: n.message
				});
			}), i.on("close", (n) => {
				c = null, e.sender.send("run:done", {
					ok: n === 0,
					exitCode: n,
					stdout: a,
					stderr: o
				}), t({
					ok: n === 0,
					exitCode: n,
					stdout: a,
					stderr: o
				});
			});
		});
	}), n.handle("openwork:runCancel", async () => c ? (c.kill("SIGINT"), { ok: !0 }) : { ok: !1 });
}
t.whenReady().then(() => {
	f(), d(), t.on("activate", () => {
		e.getAllWindows().length === 0 && d();
	});
}), t.on("window-all-closed", () => {
	process.platform !== "darwin" && t.quit();
});
//#endregion
export {};
