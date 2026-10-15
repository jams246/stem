# stem

**S**tructural **T**ypeScript **E**valuation and **M**easurement

stem is a cli tool that attempts to apply a deterministic score of how easy your TypeScript code is to read for humans. 
It does this by reading your code on 5 aspects and applying a formula to each. 

At the end of the run it gathers up the total score and presents it to you in an easy to read format, or in JSON format for your AI coding agent. 

The primary goal of stem is to help your AI coding agent hopefully have a more consistent style of TypeScript code output across your entire codebase instead of just outputting any style of code to just deliver a feature asap.

## Caveat

stem is **not** the definitive answer to code quality and readability, and you are correct to disagree with what aspects it measures, how it measures it, and how it affects the outputted code.

As humans, we all interpret code differently and what is easy for you to follow might be difficult for someone else to follow. 

## Measurements

stem measures five aspects of each function, where a total higher score is better.

### 1. Cognitive complexity: how hard the logic is to follow

Counts decisions and breaks in the code flow, such as conditions, loops and error handling. Multiple decisions in a row means the reader must keep both in mind. A flat `switch` counts once, rather than once per case.

**Research:** Muñoz Barón, Wyrich and Wagner (2020) found a link with reading time and perceived difficulty across about 24,000 evaluations of 427 code snippets. Later work by Lenarduzzi and colleagues (2023) found it was a poor predictor for junior developers. It is a useful signal, not a complete measure of understanding. [Validation study](https://arxiv.org/abs/2007.12520), [later study](https://arxiv.org/abs/2303.07722v1).

### 2. Length: how much code a function contains

Counts lines containing code, excluding blank lines and lines containing only comments. Code inside a nested function belongs to that function, so it is not counted twice.

**Research:** Peitek and colleagues (2021) studied 19 people reading code using brain imaging. They found that code size affects attention and mental effort. Length alone cannot tell a long, simple function from a long, tangled one. [Program Comprehension and Code Complexity Metrics](https://web.eecs.umich.edu/~weimerw/481/readings/peitek2021-metrics.pdf).

### 3. Parameters: how many inputs a function takes

Counts the parameters in the function's definition, including optional ones. An object unpacked into several names still counts as one input; a rest parameter also counts as one. TypeScript's special `this` parameter does not count.

**Research:** Cowan (2001) found that short-term memory holds about four chunks of information under controlled conditions. This gives an indirect reason to keep input lists small, but it does not prove a four-parameter limit or validate stem's thresholds. [The magical number 4 in short-term memory](https://pubmed.ncbi.nlm.nih.gov/11515286/).

### 4. Vocabulary: how many names the reader must track

Counts different names used in a function, including variables, called functions and properties. Repeated uses of a name count once. Names used only in type annotations do not count.

**Research:** Peitek and colleagues (2021) linked vocabulary size to demands on working memory. Their study used 19 participants and short code examples; it supports the idea, not stem's exact counting rules. This measure cannot tell whether a name is clear or misleading. [Brain-imaging study](https://web.eecs.umich.edu/~weimerw/481/readings/peitek2021-metrics.pdf).

### 5. Usage gap: how far apart a variable's uses are

Counts the statements between consecutive references to a variable (including its declaration), and reports the largest gap in the function. For example, if you declare a variable amd then immediately have 10 other unrelated statements. Blank lines and comments do not affect it. Parameters and variables from outside the function are excluded.

**Research:** Peng and Wallace's *Software Error Analysis* (NIST, 1993, page 5-13) describes variable span as the statements between successive references. stem adapts this idea by taking the largest gap. Its ability to predict understanding in TypeScript has not been validated. A large gap can be reasonable, such as a timer started at the beginning of a function. [NIST report](https://nvlpubs.nist.gov/nistpubs/Legacy/SP/nistspecialpublication500-209.pdf).

## How the score works

4 bands are present in the tool:
1. Low
2. Moderate
3. High
4. Very high

Each measurement from an aspect falls into one of these bands, e.g., functions with many lines of code carry more weight. 
Each of the five scores starts at 100: every 1% of code in the moderate band costs 0.5 points, high costs 2, and very high costs 4. Scores stop at zero. 

The overall score is the average of the five.

## How were the scores and bands decided?

The defaults were checked against 30 open-source TypeScript projects with at least 500 stars. 

The band boundaries and scoring weights are design choices and must not be considered as proven limits on what you can understand. **Use the score to guide a code review, not replace it.**

## How to use

With `stem` available in your terminal, run these commands from your TypeScript project folder:

```sh
# Measure the current code and save a starting point.
stem scan -o baseline.json

# After editing your code, compare it with that starting point.
stem compare baseline.json
```

`scan` prints a report; `-o` also saves the measurements. `compare` uses the settings saved in that file so both runs use the same rules. Keep the original baseline until you finish comparing.

Other examples:

```sh
# Scan a specific project folder.
stem scan ./my-project

# Leave out files matching a pattern when saving the baseline.
stem scan -o baseline.json --ignore '**/generated/**'

# Get JSON output for scripts or AI coding agents.
stem compare baseline.json --json

# Explain a measurement or the overall score.
stem explain usage-gap
stem explain score
```

### Exit codes when using `stem compare`

| Exit code | Interpretation                                                  |
|-----------|-----------------------------------------------------------------|
| 0         | No decrease in readability detected from baseline after changes |
| 1         | The change made code harder to follow                           |
| 2         | Compare could not complete                                      |

This lets automated AI coding agent checks stop on a regression. `--ignore` is only accepted by `scan`; `compare` reuses the saved settings.

### Example scan output

This output comes from the small project used in stem's tests:

```text
stem scan

Overall        94.3  grade A
  cognitive complexity  85.7   how hard the logic is to follow
  length                100.0  lines of code in the function
  parameters            100.0  values every caller must pass, in order
  vocabulary            100.0  different names the reader must keep track of
  usage gap             85.7   most statements between consecutive references to a local variable
Functions      17
Files          5 scanned, 0 ignored

Most to gain (1)
  parseOrder  src/orders/parse.ts:3
    cognitive complexity 11 (moderate, aim for 6 or less): how hard the logic is to follow
    usage gap 6 (moderate, aim for 3 or less): most statements between consecutive references to a local variable
    order: references at lines 7 and 16; declared at line 7.
    if, else and switch branches add 5 of the 11 points, at lines 5-17.
    Replace the chain with a lookup object, or move each decision into its own well-named function.

Learn what a metric measures: stem explain <metric>
```

The summary shows scores out of 100. The function details show the actual measurements and places to review.

### Example comparison output

After adding nested conditions to `applyDiscounts` in that project:

```text
stem compare baseline.json

Changed code   100.0 -> 80.0  (-20.0)   1 changed, 0 added, 0 removed
Overall        94.3 -> 84.9  (-9.4)    grade A -> B
  cognitive complexity  85.7 -> 36.5    (-49.2)
  length                100.0 -> 100.0  (+0.0)
  parameters            100.0 -> 100.0  (+0.0)
  vocabulary            100.0 -> 100.0  (+0.0)
  usage gap             85.7 -> 87.8    (+2.1)
Functions      17 -> 17      (+0)

Harder to follow (1)
  applyDiscounts  src/cart/total.ts:9
    cognitive complexity 3 -> 24 (high, aim for 20 or less): how hard the logic is to follow
    length 9 -> 21 (low): lines of code in the function
    vocabulary 4 -> 8 (low): different names the reader must keep track of
    Nesting adds 15 of the 24 points, at lines 12-20.
    Return early instead of nesting, so the main path is not indented.

Result: harder to follow. applyDiscounts entered the high band for cognitive complexity.
  The changed-code score fell from 100.0 to 80.0.
Learn what a metric measures: stem explain <metric>
```
