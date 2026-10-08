# HTML entity rendering (#173)

Manual fixture for Rendered/Split view — entities should show as glyphs, not raw `&amp;` or `«HTML»`.

## Common named entities

&amp; &lt; &gt; &quot; &nbsp; &rarr; &larr; &reg; &copy; &mdash; &ndash; &hellip;

## Fixture line (task verification)

&amp; &rarr; &reg;

Expected glyphs: `& → ®`

## Numeric forms

&#38; &#8594; &#174;

## Unknown entity (stays literal)

&foo;

## Still passthrough (not entity-only)

<span>tag</span>
