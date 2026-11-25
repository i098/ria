import { passwordManagerOf } from "./pluginPasswordManager.ts";

// `passwordManagerOf` adds to `problems`: the cases record the result and those problems.
export function passwordManagerOfCase(value: unknown) {
  const problems: string[] = [];
  const result = passwordManagerOf(problems, value);
  return { result, problems };
}
