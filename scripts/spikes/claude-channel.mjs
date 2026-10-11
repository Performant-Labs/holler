#!/usr/bin/env node
// A minimal Claude Code "channel" MCP server for spike #830 (no dependencies).
//
// Claude Code spawns it over stdio (it is named in --mcp-config and in
// --dangerously-load-development-channels server:hub). It declares the
// experimental `claude/channel` capability and one two-way tool, `reply`.
// A hub-like process pushes a message into the session by writing one line to the unix
// socket named by CHANNEL_SOCK; the line becomes a `notifications/claude/channel` event.
// Everything the session sends back (the `reply` tool) is appended to CHANNEL_LOG.
//
// Env (names only): CHANNEL_SOCK, CHANNEL_LOG.
import net from 'node:net';
import fs from 'node:fs';
import readline from 'node:readline';

const SOCK = process.env.CHANNEL_SOCK;
const LOG = process.env.CHANNEL_LOG;
if (!SOCK || !LOG) { console.error('CHANNEL_SOCK and CHANNEL_LOG are required'); process.exit(2); }

const log = (obj) => fs.appendFileSync(LOG, JSON.stringify({ at: Date.now(), ...obj }) + '\n');
const send = (obj) => process.stdout.write(JSON.stringify(obj) + '\n');

let n = 0;
const rl = readline.createInterface({ input: process.stdin });
rl.on('line', (line) => {
  let msg;
  try { msg = JSON.parse(line); } catch { return; }
  if (msg.method === 'initialize') {
    log({ ev: 'initialize', client: msg.params?.clientInfo?.name });
    send({
      jsonrpc: '2.0', id: msg.id,
      result: {
        protocolVersion: msg.params?.protocolVersion ?? '2025-03-26',
        capabilities: { experimental: { 'claude/channel': {} }, tools: {} },
        serverInfo: { name: 'hub', version: '0.0.1' },
        instructions: 'Messages arrive as <channel source="hub" ...>. Answer by calling the reply tool with the text.',
      },
    });
  } else if (msg.method === 'tools/list') {
    send({
      jsonrpc: '2.0', id: msg.id,
      result: { tools: [{
        name: 'reply',
        description: 'Send a message back to the hub',
        inputSchema: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] },
      }] },
    });
  } else if (msg.method === 'tools/call') {
    log({ ev: 'reply', args: msg.params?.arguments });
    send({ jsonrpc: '2.0', id: msg.id, result: { content: [{ type: 'text', text: 'sent' }] } });
  } else if (msg.method === 'ping') {
    send({ jsonrpc: '2.0', id: msg.id, result: {} });
  } else if (msg.method && msg.id !== undefined) {
    send({ jsonrpc: '2.0', id: msg.id, error: { code: -32601, message: 'method not found' } });
  } else if (msg.method) {
    log({ ev: 'notification-from-client', method: msg.method });
  }
});

try { fs.unlinkSync(SOCK); } catch { /* none */ }
net.createServer((c) => {
  readline.createInterface({ input: c }).on('line', (text) => {
    n += 1;
    log({ ev: 'push', n, text });
    send({ jsonrpc: '2.0', method: 'notifications/claude/channel', params: { content: text, meta: { n: String(n) } } });
  });
}).listen(SOCK, () => log({ ev: 'listening' }));
process.stdin.on('end', () => process.exit(0));
