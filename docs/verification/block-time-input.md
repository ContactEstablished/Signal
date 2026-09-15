# Friendly block time entry

Implemented September 14, 2026 in development only. The live installation was not rebuilt, installed, restarted, or migrated.

Start and End now accept `2pm`, `2 PM`, `2:30pm`, `2:30`, and explicit 24-hour times such as `14:30`. Without a suffix, hours 7–11 use AM and 12–6 use PM. Explicit AM/PM overrides the default. Leading-zero values such as `02:30` preserve early-morning 24-hour input and existing saved block times. `24:00` remains the end-of-day boundary.

Valid entries are displayed as `HH:mm` on blur and save. The restrictive HTML pattern and numeric keyboard hint were removed so the browser no longer blocks AM/PM entries before the app can interpret them. Invalid entries retain the typed text and produce a field-specific example. Duration buttons and time nudges use the same parser. Existing quarter-hour scheduling, date boundaries, and daylight-saving offset validation still apply.

Verification: `pnpm check` passed with zero errors/warnings; 24 focused tests passed across planner time parsing, the block editor, planner rules, and Your Day; `pnpm build` passed with the existing chunk-size advisory. Tests include actual form validity/submission, the user's examples, blur formatting, explicit AM input, duration/nudges, invalid ranges, malformed input, existing quarter-hour values, and daylight-saving offset handling. No visual inspection is claimed.

The current installed version still expects entries such as `14:00` and `14:30` until the user authorizes a live update.
