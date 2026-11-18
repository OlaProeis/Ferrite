# TeX Math Support Plan

> **Project:** Add LaTeX/TeX math rendering to Ferrite  
> **Status:** Partially superseded for preview — see below  
> **Target Version:** Preview in v0.3.2; palette / editing / export typesetting remain v0.4.0  
> **Created:** 2025-01-12

> **v0.3.2 preview slice:** Do **not** implement the from-scratch TeX box model in this document for Rendered/Split preview. That work is specified in [`docs/ai-workflow/prds/prd-v0.3.2-math-preview.md`](ai-workflow/prds/prd-v0.3.2-math-preview.md) and implemented as described in [`docs/technical/markdown/math-preview.md`](technical/markdown/math-preview.md) (`pulldown-latex` → `formulary` → rasterized texture). Keep this plan for longer-term goals (symbol palette, formula editing, PDF/HTML math).

## Executive Summary

Add native TeX math rendering to Ferrite, enabling users to write and preview mathematical formulas using standard LaTeX syntax (`$...$` for inline, `$$...$$` for display). This addresses the most significant feature gap compared to Typora and makes Ferrite suitable for academic and technical writing.

**Constraint:** Pure Rust implementation required. No JavaScript runtimes (KaTeX, MathJax).

---

## Motivation

### User Feedback

> "The main feature Ferrite is currently lacking to be used for most (all?) applications where I use Typora is a way to easily write and render maths formula."

### Use Cases

1. **Academic writing** - Papers, thesis drafts, lecture notes
2. **Technical documentation** - Algorithm descriptions, specifications
3. **Educational content** - Tutorials, textbooks
4. **Scientific note-taking** - Research notes, lab journals

### Competitive Analysis

| Editor | Math Support | Implementation |
|--------|-------------|----------------|
| Typora | Full TeX | KaTeX (JS) |
| Obsidian | Full TeX | MathJax (JS) |
| VS Code | Plugin-based | KaTeX (JS) |
| Zettlr | Full TeX | KaTeX (JS) |
| **Ferrite** | **Planned** | **Pure Rust** |

Ferrite would be unique as a pure-Rust Markdown editor with native math rendering.

---

## Current State

### Parser Support (Ready)

Comrak (our Markdown parser) already supports math syntax:

```rust
// Just need to enable this option
options.extension.math_dollars = true;
```

This parses:
- `$E = mc^2$` → inline math
- `$$\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}$$` → display math

### What's Missing

1. **AST node type** for math expressions
2. **LaTeX parser** to convert TeX syntax → internal representation
3. **Layout engine** for mathematical typesetting
4. **Glyph rendering** for math symbols
5. **egui integration** to display rendered math

---

## Technical Challenges

### The Complexity of Math Typesetting

Mathematical typesetting is significantly more complex than regular text:

| Challenge | Description | Example |
|-----------|-------------|---------|
| **Vertical layout** | Fractions, limits stack vertically | $\frac{a}{b}$ |
| **Subscripts/superscripts** | Smaller, positioned relative to baseline | $x^2$, $a_n$ |
| **Variable-height delimiters** | Parentheses scale to content | $\left(\frac{a}{b}\right)$ |
| **Radicals** | Square root signs scale | $\sqrt{x^2 + y^2}$ |
| **Matrices** | Grid alignment | $\begin{bmatrix} a & b \\ c & d \end{bmatrix}$ |
| **Operators** | Special spacing rules | $\sin x$, $\lim_{x \to 0}$ |
| **Fonts** | Math-specific glyphs | $\mathbb{R}$, $\mathcal{L}$, $\alpha$ |

### TeX Spacing Rules

TeX has complex spacing rules based on atom types:

| Between | Spacing |
|---------|---------|
| Ord + Op | Thin space |
| Op + Ord | Thin space |
| Bin + Ord | Medium space |
| Rel + Ord | Thick space |
| Open + anything | No space |

Getting these right is essential for professional-looking output.

---

## Existing Rust Ecosystem

### Available Crates

| Crate | Purpose | Limitation |
|-------|---------|------------|
| `latex2mathml` | LaTeX → MathML | No rendering, just conversion |
| `katex` | LaTeX → HTML | Embeds JavaScript runtime |
| `mathml` | MathML parser | No LaTeX input |
| `unicode-math-class` | TeX atom classification | Helper only |

### Gap Analysis

**No pure-Rust crate renders LaTeX math to graphics.**

The closest approach requires:
1. `latex2mathml` for parsing
2. Custom MathML → egui renderer

Or build from scratch:
1. Custom LaTeX parser
2. Custom layout engine
3. Custom egui renderer

---

## Architecture

### Proposed Module Structure

```
src/
├── math/
│   ├── mod.rs              # Public API
│   ├── parser.rs           # LaTeX → MathAST
│   ├── ast.rs              # Math expression AST types
│   ├── layout.rs           # Box layout algorithm
│   ├── atoms.rs            # TeX atom types and spacing
│   ├── fonts.rs            # Math font handling
│   └── render.rs           # egui rendering
```

### Core Types

```rust
/// Mathematical expression AST
pub enum MathNode {
    /// Single symbol (letter, digit, operator)
    Symbol { char: char, atom_type: AtomType },
    /// Horizontal sequence of nodes
    Row(Vec<MathNode>),
    /// Fraction: numerator over denominator
    Fraction { num: Box<MathNode>, denom: Box<MathNode> },
    /// Superscript
    Superscript { base: Box<MathNode>, sup: Box<MathNode> },
    /// Subscript
    Subscript { base: Box<MathNode>, sub: Box<MathNode> },
    /// Both sub and superscript
    SubSup { base: Box<MathNode>, sub: Box<MathNode>, sup: Box<MathNode> },
    /// Square root or nth root
    Radical { index: Option<Box<MathNode>>, radicand: Box<MathNode> },
    /// Delimited expression (parentheses, brackets, etc.)
    Delimited { left: char, right: char, content: Box<MathNode> },
    /// Matrix or array
    Matrix { rows: Vec<Vec<MathNode>>, delimiters: (char, char) },
    /// Big operator (sum, integral, etc.)
    BigOp { symbol: char, limits: bool, sub: Option<Box<MathNode>>, sup: Option<Box<MathNode>> },
    /// Text in math mode
    Text(String),
    /// Spacing
    Space(SpaceWidth),
}

/// TeX atom types for spacing calculation
pub enum AtomType {
    Ordinary,     // Variables, constants
    Binary,       // +, -, ×
    Relation,     // =, <, >
    Opening,      // (, [, {
    Closing,      // ), ], }
    Punctuation,  // ,, ;
    Operator,     // sin, cos, lim
    Inner,        // Fractions
}
```

### Layout Algorithm

Based on TeX's box model:

```rust
/// A laid-out math element with dimensions
pub struct MathBox {
    /// Width of the box
    pub width: f32,
    /// Height above baseline
    pub ascent: f32,
    /// Depth below baseline
    pub descent: f32,
    /// Child boxes with relative positions
    pub children: Vec<PositionedBox>,
}

/// Position a child box relative to parent origin
pub struct PositionedBox {
    pub x: f32,
    pub y: f32,  // Relative to baseline
    pub content: MathBoxContent,
}

pub enum MathBoxContent {
    Glyph { char: char, font_size: f32 },
    Rule { width: f32, height: f32 },  // Fraction bars
    Box(MathBox),
}
```

### Rendering Pipeline

```
LaTeX String
    │
    ▼
┌─────────────────┐
│  LaTeX Parser   │  Parse TeX commands, handle macros
└────────┬────────┘
         │
         ▼
    MathNode AST
         │
         ▼
┌─────────────────┐
│ Layout Engine   │  Calculate boxes, positions, sizes
└────────┬────────┘
         │
         ▼
    MathBox Tree
         │
         ▼
┌─────────────────┐
│  egui Renderer  │  Draw glyphs, lines, curves
└────────┬────────┘
         │
         ▼
   Rendered Output
```

---

## Implementation Phases

### Phase 0: Preparation (v0.3.x) - 1 day

**Goal:** Enable math parsing without rendering

- [ ] Enable `math_dollars` in comrak options
- [ ] Add `MarkdownNodeType::Math { content, display }` to AST
- [ ] Show placeholder in preview: `[Math: E = mc^2]`
- [ ] Document that full rendering is coming in v0.4.0

**Deliverable:** Math syntax recognized, users know it's planned

### Phase 1: LaTeX Parser (2-3 weeks)

**Goal:** Parse LaTeX math into AST

- [ ] Tokenizer for LaTeX syntax
- [ ] Parser for basic expressions (symbols, operators)
- [ ] Superscripts and subscripts (`^`, `_`)
- [ ] Fractions (`\frac{}{}`)
- [ ] Greek letters (`\alpha`, `\beta`, etc.)
- [ ] Common operators (`\sin`, `\cos`, `\sum`, `\int`)
- [ ] Delimiters and `\left`/`\right`
- [ ] Square roots (`\sqrt{}`, `\sqrt[]{}`)
- [ ] Text mode (`\text{}`)
- [ ] Basic matrices (`\begin{matrix}...\end{matrix}`)
- [ ] Error recovery for malformed input

**Parser coverage target:** 80% of common LaTeX math

### Phase 2: Layout Engine (2-3 weeks)

**Goal:** Convert AST to positioned boxes

- [ ] Implement box model types
- [ ] Basic horizontal layout (Row)
- [ ] Fraction layout with proper bar thickness
- [ ] Superscript/subscript positioning (cramped styles)
- [ ] Radical sign sizing and positioning
- [ ] Delimiter scaling algorithm
- [ ] TeX spacing rules between atoms
- [ ] Display vs inline style handling
- [ ] Text measurement integration

**Reference:** The TeXbook, Appendix G (math typesetting rules)

### Phase 3: Font & Glyph Handling (1-2 weeks)

**Goal:** Proper math font support

- [ ] Math symbol coverage (Greek, operators, arrows)
- [ ] Variable-height delimiter glyphs
- [ ] Radical sign construction (from pieces)
- [ ] Font size scaling for sub/superscripts
- [ ] Bold math (`\mathbf`)
- [ ] Calligraphic (`\mathcal`)
- [ ] Blackboard bold (`\mathbb`)
- [ ] Integration with Ferrite's font system

**Font options:**
- STIX Two Math (open source, comprehensive)
- Latin Modern Math (TeX standard)
- Embedded subset of essential glyphs

### Phase 4: egui Integration (1-2 weeks)

**Goal:** Render math in Ferrite preview

- [ ] MathBox → egui Painter calls
- [ ] Inline math rendering (within text flow)
- [ ] Display math rendering (centered block)
- [ ] Theme-aware colors (light/dark)
- [ ] Click-to-edit behavior (switch to raw LaTeX)
- [ ] Error display for invalid LaTeX
- [ ] Caching for performance

### Phase 5: WYSIWYG Enhancement (1-2 weeks)

**Goal:** Rich editing experience (depends on FerriteEditor from v0.3.0)

- [ ] Inline math preview while typing (Typora-style)
- [ ] Math input assistance (symbol palette?)
- [ ] Auto-preview after typing `$`
- [ ] Cursor navigation through math

### Phase 6: Polish & Documentation (1 week)

**Goal:** Production-ready release

- [ ] Performance optimization
- [ ] Comprehensive testing
- [ ] Supported LaTeX reference documentation
- [ ] User guide for math features
- [ ] Update README and screenshots

---

## Supported LaTeX Subset (Target)

### Tier 1: Essential (Must Have)

```latex
% Fractions
\frac{a}{b}

% Superscripts and subscripts
x^2, x_i, x_i^2

% Greek letters
\alpha, \beta, \gamma, \delta, \epsilon, \pi, \sigma, \omega...

% Common operators
+, -, \times, \div, \cdot, \pm

% Relations
=, \neq, <, >, \leq, \geq, \approx

% Big operators
\sum_{i=1}^{n}, \prod, \int_a^b, \lim_{x \to 0}

% Square roots
\sqrt{x}, \sqrt[3]{x}

% Parentheses
(, ), [, ], \{, \}, \langle, \rangle
\left( \frac{a}{b} \right)

% Text in math
\text{where } x > 0
```

### Tier 2: Important (Should Have)

```latex
% Matrices
\begin{matrix} a & b \\ c & d \end{matrix}
\begin{pmatrix}...\end{pmatrix}  % parentheses
\begin{bmatrix}...\end{bmatrix}  % brackets

% Aligned equations
\begin{align} ... \end{align}

% Font styles
\mathbf{v}, \mathit{x}, \mathrm{d}x

% Accents
\hat{x}, \bar{x}, \vec{v}, \dot{x}

% More functions
\sin, \cos, \tan, \log, \ln, \exp, \max, \min
```

### Tier 3: Nice to Have (Could Defer)

```latex
% Blackboard bold
\mathbb{R}, \mathbb{N}, \mathbb{Z}

% Calligraphic
\mathcal{L}, \mathcal{O}

% Advanced constructs
\underbrace{...}_{text}
\overbrace{...}^{text}

% Cases
\begin{cases} ... \end{cases}

% More symbols
\infty, \partial, \nabla, \forall, \exists
```

---

## Technical Decisions

### Decision 1: LaTeX Parser Approach

**Options:**
1. Use `latex2mathml` and parse MathML
2. Write custom LaTeX parser
3. Port existing parser (e.g., from KaTeX source)

**Recommendation:** Custom parser (Option 2)
- More control over error handling
- Can target specific subset
- No dependency on MathML intermediate format
- Better integration with our AST types

### Decision 2: Font Strategy

**Options:**
1. Embed full math font (STIX Two Math: ~1.5MB)
2. Embed minimal glyph subset (~200KB)
3. Use system fonts with fallback

**Recommendation:** Embed minimal subset (Option 2)
- Consistent rendering across platforms
- Manageable binary size increase
- Cover 95% of use cases

### Decision 3: Display in Editor vs Preview Only

**Options:**
1. Preview/Split mode only (show raw in Raw mode)
2. Inline preview in Raw mode (like Typora)

**Recommendation:** Start with Option 1, add Option 2 after FerriteEditor
- Option 1 works with current egui TextEdit
- Option 2 requires custom editor (v0.3.0)

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Layout complexity underestimated | High | Start with simple cases, iterate |
| Font rendering issues | Medium | Test early with math fonts |
| Performance (complex equations) | Medium | Implement caching, lazy rendering |
| Scope creep (more LaTeX) | Medium | Define clear tier boundaries |
| Competition releases first | Low | Unique as pure-Rust + full editor |

---

## Success Criteria

### Minimum Viable

- [ ] Basic math renders correctly (`$x^2$`, `$\frac{a}{b}$`)
- [ ] Common Greek letters and operators work
- [ ] Display vs inline renders differently
- [ ] No panics on malformed input
- [ ] Acceptable performance (<100ms for complex equations)

### Full Success

- [ ] Tier 1 and Tier 2 LaTeX fully supported
- [ ] Rendering quality comparable to KaTeX
- [ ] Inline preview in editor (Typora-style)
- [ ] Users can migrate from Typora for academic writing

---

## Dependencies

### Requires v0.3.0

- **FerriteEditor widget** - For inline math preview in editor
- **Modular architecture** - Clean separation of math module

### External Dependencies (New)

```toml
# Potentially needed
unicode-math-class = "0.1"  # TeX atom classification
```

---

## Related Documents

- [Mermaid Crate Plan](./mermaid-crate-plan.md) - Similar rendering architecture
- [Custom Editor Plan](./technical/planning/custom-editor-widget-plan.md) - Enables WYSIWYG math
- [Modular Refactor Plan](./refactor.md) - Feature flag architecture

---

## Future: ferrite-math Crate?

Similar to the Mermaid crate extraction, the math rendering engine could potentially become a standalone crate:

- **ferrite-math** or **tex-render**
- Backend-agnostic (egui, SVG, PNG)
- Useful for mdBook, documentation tools, etc.

This would be evaluated after v0.4.0 based on implementation quality and community interest.

---

## Changelog

| Date | Change |
|------|--------|
| 2025-01-12 | Initial planning document |
