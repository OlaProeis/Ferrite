# Emoji font fallback (#168)

Manual fixture for the OS emoji font fallback. Open in **Raw**, **Rendered**, and
**Split** — every emoji below should render as a glyph (color or monochrome),
never as an empty box / tofu (`□`).

## Common single-codepoint emoji

😀 😅 🤔 😴 🥳 😢 😡

## Symbols & pictographs

✅ ❌ ⚠️ ⭐ 🔥 💡 📝 📁 🔒 🔍 ⏰ 🎉

## Skin tone modifiers

👍 👍🏻 👍🏽 👍🏿 👋🏼

## ZWJ sequences (may degrade to components — must not crash)

👨‍👩‍👧 👨‍💻 🏳️‍🌈 🧑‍🚀

## Flags (regional indicator pairs)

🇳🇴 🇺🇸 🇯🇵 🇩🇪

## Emoji inside markdown structures

- List item with emoji ✅ done
- **Bold with emoji 🔥 inside**
- `inline code with emoji ⚠️`

| Status | Icon |
|--------|------|
| Pass   | ✅   |
| Fail   | ❌   |

> Callout quote with emoji 💡 tip

```text
Code block with emoji: 🚀 (monospace fallback path)
```

## What to check

- [ ] Raw mode: all emoji visible (Monospace fallback).
- [ ] Rendered mode: all emoji visible (Proportional fallback).
- [ ] UI icons (ribbon, tabs, panels) still use Phosphor — unchanged.
- [ ] No crash on ZWJ sequences; degradation to separate glyphs is acceptable.
- [ ] CJK text still renders: 中文テスト한국어 (emoji fallback must not break CJK order).
