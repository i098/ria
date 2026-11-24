// Ria original, no Ardia source.

const { execFileSync } = require("node:child_process");

exports.replay = (actions) => JSON.parse(execFileSync(process.env.RIA_REPLAY_BIN, {
  input: JSON.stringify(actions),
  encoding: "utf8",
}));
