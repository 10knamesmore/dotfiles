import { constants, openSync, writeSync } from "node:fs";
import { connect } from "node:net";
import { ReadStream } from "node:tty";
import {
  decodeKittyPrintable,
  isKeyRelease,
  Key,
  matchesKey,
  setKittyProtocolActive,
  StdinBuffer,
} from "@earendil-works/pi-tui";

// stdout is sudo's private password pipe. The Pi socket carries only lengths and cancellation.
const channel = connect(process.env.PI_ASKPASS_SOCKET);
const keys = new StdinBuffer();
const characters = [];
let keyboard;
let finished = false;

function finish(submit = false) {
  if (finished) return;
  finished = true;
  keyboard?.destroy();
  keys.destroy();
  if (submit) writeSync(1, characters.join("") + "\n");
  characters.length = 0;
  channel.end(submit ? undefined : "cancel\n");
  process.exitCode = submit ? 0 : 1;
}

function insert(text) {
  if (finished || /\p{Cc}/u.test(text)) return;
  for (const char of text) characters.push(char);
  channel.write(`${characters.length}\n`);
}

keys.on("paste", insert);
keys.on("data", (key) => {
  if (finished || isKeyRelease(key)) return;
  if (matchesKey(key, Key.escape) || matchesKey(key, Key.ctrl("c"))) return finish();
  if (matchesKey(key, Key.enter)) return finish(true);
  if (matchesKey(key, Key.backspace)) characters.pop();
  else if (matchesKey(key, Key.ctrl("u"))) characters.length = 0;
  else return insert(decodeKittyPrintable(key) ?? key);
  channel.write(`${characters.length}\n`);
});
channel.on("error", () => finish());
channel.on("close", () => finish());
channel.on("end", () => finish());
channel.once("data", (protocol) => {
  setKittyProtocolActive(protocol.toString() === "K");
  keyboard = new ReadStream(openSync(process.env.PI_ASKPASS_TTY, constants.O_RDONLY | constants.O_NOCTTY));
  keyboard.setEncoding("utf8");
  keyboard.on("data", (data) => keys.process(data));
  keyboard.on("error", () => finish());
  keyboard.on("end", () => finish());
});
