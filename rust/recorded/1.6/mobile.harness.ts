import { ArdiaError, ardiaError } from "./errors.ts";
import * as mobile from "./mobile.ts";

const calls = {
  mobileLoadBangDocument: mobile.mobileLoadBangDocument,
  mobileBangDocument: mobile.mobileBangDocument,
  mobileSetSources: mobile.mobileSetSources,
  mobileSetSearch: mobile.mobileSetSearch,
  mobileEngines: mobile.mobileEngines,
  mobileWarmBangs: mobile.mobileWarmBangs,
  mobileSuggest: mobile.mobileSuggest,
  mobileResolve: mobile.mobileResolve,
  mobileSuggestRequest: mobile.mobileSuggestRequest,
  mobileParseRemote: mobile.mobileParseRemote,
  mobileFindScope: mobile.mobileFindScope,
  mobileClean: mobile.mobileClean,
  mobileDisplay: mobile.mobileDisplay,
  ardiaError: (message: string, code?: string) => {
    const { name, code: resultCode, message: resultMessage } = ardiaError(
      code === undefined ? new Error(message) : new ArdiaError(code, message)
    );
    return { name, code: resultCode, message: resultMessage };
  },
};

export function replayMobile(input: string): unknown[] {
  const steps: [keyof typeof calls, ...never[]][] = JSON.parse(input);
  return steps.map(([name, ...args]) => {
    try {
      return (calls[name] as (...args: never[]) => unknown)(...args);
    } catch (error) {
      if ((error as Error).name !== "TypeError") throw error;
      return { error: (error as Error).message };
    }
  });
}
