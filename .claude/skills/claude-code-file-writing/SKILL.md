---
name: claude-code-file-writing
description: >
  Prevents "Invalid tool parameters" and "Error writing file" errors in Claude Code.
  Use this skill whenever Claude Code needs to create, write, edit, or save any file.
  Triggers on: writing code files, creating configs, saving outputs, editing existing files,
  any task that produces a file artifact. If Claude Code has been getting file write errors,
  consult this skill immediately before attempting again.
---

# Claude Code — File Writing (Correct Patterns)

This skill exists because Claude Code has a history of producing `Invalid tool parameters`
or `Error writing file` when writing files. Follow these rules exactly.

---

## 1. Tool Selection

| Situation | Correct Tool |
|---|---|
| File does not exist yet | `Write` |
| File exists, replace a known unique string | `Edit` (str_replace mode) |
| File exists, replace entire content | `Write` (safe to overwrite) |
| Need to create a directory first | `Bash` → `mkdir -p <dir>` → then `Write` |
| Appending content | `Bash` → `echo "..." >> file` or `Write` full new content |

**Never** use `Edit` on a file that doesn't exist yet — it will error.  
**Never** use `Write` with a path whose parent directory doesn't exist — it will error.

---

## 2. Write Tool — Required Parameters

```
tool: Write
parameters:
  file_path: <absolute or relative path>   ← REQUIRED, exact spelling
  content: <full file content as string>   ← REQUIRED, exact spelling
```

Common mistakes that cause `Invalid tool parameters`:
- Using `path` instead of `file_path`
- Using `text` or `body` instead of `content`
- Leaving `content` empty or undefined
- Passing an array instead of a string for `content`

---

## 3. Edit Tool — Required Parameters

```
tool: Edit
parameters:
  file_path: <path to existing file>    ← REQUIRED
  old_str: <exact substring to replace> ← REQUIRED, must exist exactly once
  new_str: <replacement string>         ← REQUIRED (can be "" to delete)
```

Common mistakes:
- `old_str` doesn't match file content exactly (whitespace, line endings)
- `old_str` matches more than once — make it longer/more unique
- File doesn't exist yet — use `Write` instead
- Using `old_text`/`new_text` instead of `old_str`/`new_str`

---

## 4. Directory Must Exist Before Writing

Always check / create the parent directory first:

```bash
# Step 1 — ensure directory exists
mkdir -p /path/to/directory

# Step 2 — then write the file
# (use Write tool with file_path: /path/to/directory/file.ext)
```

If you skip this, `Write` will produce `Error writing file: ENOENT`.

---

## 5. Path Rules

- Prefer **absolute paths** to avoid ambiguity.
- Never use `~` — expand it to `/home/<user>` or use `$HOME` in Bash only.
- On macOS, home is typically `/Users/<username>`; on Linux `/home/<username>`.
- Avoid spaces in paths unless you quote them in Bash.
- Confirm the target directory is writable (not `/mnt/skills`, not read-only mounts).

---

## 6. Large Files — Chunked Writing

For files > ~200 lines, write the complete content in one `Write` call.  
Do **not** attempt to write a file in multiple partial `Write` calls — each call
overwrites the whole file, losing previous content.

If you need to build content incrementally, accumulate it in a variable/string first,
then make a single `Write` call with the complete content.

---

## 7. Encoding & Special Characters

- `content` must be a valid UTF-8 string.
- Escape backslashes and quotes properly if constructing content programmatically.
- For binary files, use `Bash` with `base64 -d` rather than the `Write` tool.
- Multiline strings: use a literal newline character (`\n`), not the two-character sequence `\\n`.

---

## 8. Verification After Writing

After every file write, confirm success with a quick `Bash` check:

```bash
# Confirm file exists and has content
ls -lh /path/to/file && head -5 /path/to/file
```

If the file is empty or missing, the write silently failed — retry.

---

## 9. Quick Diagnostic Checklist

When you get `Invalid tool parameters`:
1. ✅ Is the tool name spelled correctly? (`Write`, `Edit`, `Bash`)
2. ✅ Is `file_path` spelled exactly as `file_path`?
3. ✅ Is `content` a non-empty string?
4. ✅ For Edit: is `old_str` spelled `old_str` (not `old_text`)?

When you get `Error writing file`:
1. ✅ Does the parent directory exist? (Run `mkdir -p` first)
2. ✅ Is the path absolute and valid (no `~`, no typos)?
3. ✅ Is the target location writable (not a read-only mount)?
4. ✅ Is the `content` value a string, not `undefined` or `null`?

---

## 10. Full Working Example

```
# Create directory
Bash: mkdir -p /home/user/project/src

# Write a new file
Write:
  file_path: /home/user/project/src/main.py
  content: |
    def hello():
        print("Hello, world!")

    if __name__ == "__main__":
        hello()

# Edit a specific line
Edit:
  file_path: /home/user/project/src/main.py
  old_str: print("Hello, world!")
  new_str: print("Hello, Claude!")

# Verify
Bash: cat /home/user/project/src/main.py
```
