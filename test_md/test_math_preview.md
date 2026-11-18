# Math preview (v0.3.2)

Open in **Rendered** and **Split**. Default settings, **Render math** on. The file stays LaTeX: Raw and the Split raw pane show the dollars. Preview draws the formula.

## Inline on the baseline

Energy $E = mc^2$ is rest energy.

The formula sits on the same baseline as “Energy” and “is”. It is not the characters `E = mc^2` in the body font.

Click this paragraph, type a character, click away. The formula paints again. Undo restores the sentence.

## Display, paragraph is only the formula

$$E = mc^2$$

One centered formula. Nothing else on that row.

## Summation and fraction

$$\sum_{i=0}^{n} i = \frac{n(n+1)}{2}$$

The fraction bar is a rule, not the character `/`. Limits sit above and below the sum.

## Radical and Greek

Distance is $\sqrt{x^2 + y^2}$.

Greek from the math font: $\alpha + \beta = \gamma$.

## Currency stays text

Lunch was $20 and the taxi was $30.

Both amounts stay body text. No formula, no missing words.

## Failure

Broken: $\frac{$.

The delimited source stays visible. Hover for the failure tooltip. The app does not panic.

## Display in the middle of a sentence

Before $$E = mc^2$$ after.

The formula is its own centered row. “Before” and “after” are on the rows around it.

## Other blocks

# Heading with $E = mc^2$

- A list item with $a^2 + b^2 = c^2$
- Display in a list:

  $$x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}$$

| What | Formula |
| --- | --- |
| Inline | $E = mc^2$ |
| Broken | $\frac{$ |

## Harder formulas

These stay inside the v0.3.2 renderer. A construct it cannot lay out stays as the delimited source, with a hover tooltip. It should not disappear or panic.

Integral and nested radical:

$$\int_{0}^{\infty} e^{-x}\,dx = 1 \qquad \sqrt{\frac{a}{b+c}}$$

Limit, product, and binomial:

$$\lim_{n \to \infty}\prod_{k=1}^{n}\left(1+\frac{1}{k}\right) \qquad \binom{n}{k}=\frac{n!}{k!(n-k)!}$$

Piecewise and a matrix:

$$|x| = \begin{cases} x & x \ge 0 \\ -x & x < 0 \end{cases} \qquad \begin{pmatrix} a & b \\ c & d \end{pmatrix}$$

Double-struck, script, and accents, inline: $\forall x \in \mathbb{R},\ \hat{x}+\bar{y}+\vec{z}$.

Operator and aligned lines:

$$\left(\frac{\partial^{2}}{\partial x^{2}}+\frac{\partial^{2}}{\partial y^{2}}\right)\psi = 0$$

$$\begin{aligned} (a+b)^{2} &= a^{2}+2ab+b^{2} \\ &= a^{2}+b^{2}+2ab \end{aligned}$$

## Checks that are not a formula on this page

- **Render math** off: the first sentence shows `$E = mc^2$` as text.
- Split: preview matches Rendered. The raw pane shows the dollars.
- HTML and PDF export of the first sentence still contain `$E = mc^2$`, not MathML.
- Switch light and dark. The formula recolors. It does not keep the previous text color.
