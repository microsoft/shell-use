import {
  DEFAULT_COLS,
  DEFAULT_ROWS,
  resolveBinary,
  resolveHome,
  resolveSession,
} from "./config.js";
import { envPairs, unwrap } from "./protocol.js";
import * as transport from "./transport.js";
import { checkVersion } from "./version.js";
import type {
  Cell,
  ClientOptions,
  OpenResult,
  Shell,
  SpawnOptions,
  State,
} from "./types.js";

export interface WaitTextOptions {
  regex?: boolean;
  full?: boolean;
  not?: boolean;
  timeout?: number;
}

export interface ExpectTextOptions {
  regex?: boolean;
  full?: boolean;
  strict?: boolean;
  not?: boolean;
  fg?: string;
  bg?: string;
  timeout?: number;
}

export interface MouseButtonOptions {
  button?: number;
}

class Mouse {
  #client: ShellUse;

  constructor(client: ShellUse) {
    this.#client = client;
  }

  async click(
    x: number | null = null,
    y: number | null = null,
    opts: { onText?: string; button?: number; clicks?: number } = {},
  ): Promise<void> {
    await this.#client.send({
      kind: "mouse",
      action: {
        op: "click",
        x,
        y,
        on_text: opts.onText ?? null,
        button: opts.button ?? 0,
        clicks: opts.clicks ?? 1,
      },
    });
  }

  async move(x: number, y: number): Promise<void> {
    await this.#client.send({ kind: "mouse", action: { op: "move", x, y } });
  }

  async down(x: number, y: number, opts: MouseButtonOptions = {}): Promise<void> {
    await this.#client.send({
      kind: "mouse",
      action: { op: "down", x, y, button: opts.button ?? 0 },
    });
  }

  async up(x: number, y: number, opts: MouseButtonOptions = {}): Promise<void> {
    await this.#client.send({
      kind: "mouse",
      action: { op: "up", x, y, button: opts.button ?? 0 },
    });
  }

  async drag(
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    opts: MouseButtonOptions = {},
  ): Promise<void> {
    await this.#client.send({
      kind: "mouse",
      action: { op: "drag", x1, y1, x2, y2, button: opts.button ?? 0 },
    });
  }

  async scroll(direction: "up" | "down", opts: { amount?: number } = {}): Promise<void> {
    await this.#client.send({
      kind: "mouse",
      action: { op: "scroll", direction, amount: opts.amount ?? 3 },
    });
  }
}

export class ShellUse {
  readonly session: string;
  readonly mouse: Mouse;
  #binary: string;
  #home?: string;
  #versionChecked = false;

  constructor(session?: string, opts: ClientOptions = {}) {
    this.session = resolveSession(session);
    this.#binary = resolveBinary(opts.binary);
    this.#home = resolveHome(opts.home);
    this.mouse = new Mouse(this);
  }

  async send(payload: unknown): Promise<unknown> {
    await this.#checkVersion();
    const resp = await transport.request(this.session, this.#home, this.#binary, payload);
    return unwrap(resp);
  }

  async #checkVersion(): Promise<void> {
    if (this.#versionChecked) {
      return;
    }
    const resp = await transport.request(this.session, this.#home, this.#binary, {
      kind: "status",
    });
    const data = unwrap(resp) as { version?: string } | undefined;
    checkVersion(data?.version);
    this.#versionChecked = true;
  }

  async open(opts: SpawnOptions & { shell?: Shell } = {}): Promise<OpenResult> {
    return (await this.send({
      kind: "open",
      shell: opts.shell ?? null,
      program: null,
      cols: opts.cols ?? DEFAULT_COLS,
      rows: opts.rows ?? DEFAULT_ROWS,
      cwd: opts.cwd ?? null,
      env: envPairs(opts.env),
    })) as OpenResult;
  }

  async run(program: string, args: string[] = [], opts: SpawnOptions = {}): Promise<OpenResult> {
    return (await this.send({
      kind: "open",
      shell: null,
      program: [program, ...args],
      cols: opts.cols ?? DEFAULT_COLS,
      rows: opts.rows ?? DEFAULT_ROWS,
      cwd: opts.cwd ?? null,
      env: envPairs(opts.env),
    })) as OpenResult;
  }

  async close(): Promise<void> {
    if (!(await transport.canConnect(this.session, this.#home))) {
      return;
    }
    const resp = await transport.request(
      this.session,
      this.#home,
      this.#binary,
      { kind: "close" },
      false,
    );
    unwrap(resp);
  }

  async type(text: string): Promise<void> {
    await this.send({ kind: "write", data: text });
  }

  async write(data: string): Promise<void> {
    await this.send({ kind: "write", data });
  }

  async submit(text: string | null = null): Promise<void> {
    await this.send({ kind: "submit", data: text });
  }

  async press(...keys: string[]): Promise<void> {
    await this.send({ kind: "press", keys });
  }

  async keys(combo: string): Promise<void> {
    await this.send({ kind: "press", keys: [combo] });
  }

  async resize(cols: number, rows: number): Promise<void> {
    await this.send({ kind: "resize", cols, rows });
  }

  async signal(name: string): Promise<void> {
    await this.send({ kind: "signal", name });
  }

  async kill(): Promise<void> {
    await this.send({ kind: "signal", name: "KILL" });
  }

  async state(): Promise<State> {
    return (await this.send({ kind: "state" })) as State;
  }

  async text(opts: { full?: boolean } = {}): Promise<string> {
    const data = (await this.send({ kind: "text", full: opts.full ?? false })) as {
      text: string;
    };
    return data.text;
  }

  async cells(x: number, y: number, w = 1, h = 1): Promise<Cell[]> {
    const data = (await this.send({ kind: "cells", x, y, w, h })) as { cells: Cell[] };
    return data.cells;
  }

  async get(field: string): Promise<unknown> {
    const data = (await this.send({ kind: "get", field })) as { value: unknown };
    return data.value;
  }

  async getCommand(): Promise<string | null> {
    return (await this.get("command")) as string | null;
  }

  async getOutput(): Promise<string | null> {
    return (await this.get("output")) as string | null;
  }

  async getExitCode(): Promise<number | null> {
    return (await this.get("exit-code")) as number | null;
  }

  async getCwd(): Promise<string | null> {
    return (await this.get("cwd")) as string | null;
  }

  async getCursor(): Promise<{ x: number; y: number }> {
    return (await this.get("cursor")) as { x: number; y: number };
  }

  async getSize(): Promise<{ cols: number; rows: number }> {
    return (await this.get("size")) as { cols: number; rows: number };
  }

  async screenshot(path: string | null = null, opts: { full?: boolean } = {}): Promise<string> {
    const data = (await this.send({ kind: "screenshot", full: opts.full ?? false, path })) as {
      path?: string;
      text?: string;
    };
    return (data.path ?? data.text) as string;
  }

  async waitText(text: string, opts: WaitTextOptions = {}): Promise<void> {
    await this.send({
      kind: "wait_text",
      text,
      regex: opts.regex ?? false,
      full: opts.full ?? false,
      timeout_ms: opts.timeout ?? 5000,
      not: opts.not ?? false,
    });
  }

  async waitIdle(opts: { timeout?: number } = {}): Promise<void> {
    await this.send({ kind: "wait_idle", timeout_ms: opts.timeout ?? 5000 });
  }

  async waitCommand(opts: { timeout?: number } = {}): Promise<void> {
    await this.send({ kind: "wait_command", timeout_ms: opts.timeout ?? 30000 });
  }

  async waitExit(opts: { timeout?: number } = {}): Promise<void> {
    await this.send({ kind: "wait_exit", timeout_ms: opts.timeout ?? 30000 });
  }

  async expectText(text: string, opts: ExpectTextOptions = {}): Promise<void> {
    await this.send({
      kind: "expect_text",
      text,
      regex: opts.regex ?? false,
      full: opts.full ?? false,
      strict: opts.strict ?? true,
      not: opts.not ?? false,
      fg: opts.fg ?? null,
      bg: opts.bg ?? null,
      timeout_ms: opts.timeout ?? 5000,
    });
  }

  async expectExitCode(code: number): Promise<void> {
    await this.send({ kind: "expect_exit_code", code });
  }

  async expectOutput(text: string, opts: { regex?: boolean } = {}): Promise<void> {
    await this.send({ kind: "expect_output", text, regex: opts.regex ?? false });
  }

  async expectSnapshot(
    name: string,
    opts: { update?: boolean; includeColors?: boolean } = {},
  ): Promise<string> {
    const data = (await this.send({
      kind: "snapshot",
      name,
      update: opts.update ?? false,
      include_colors: opts.includeColors ?? false,
      cwd: process.cwd(),
    })) as { status: string };
    return data.status;
  }

  async [Symbol.asyncDispose](): Promise<void> {
    await this.close();
  }
}
