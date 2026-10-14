//! `stem explain`: fixed explanations; only the bands vary with the project.

use crate::error::StemError;
use crate::metric::{Metric, Thresholds};

/// Every topic `stem explain` accepts, besides the overview.
pub const TOPICS: [&str; 5] = [
    "cognitive-complexity",
    "length",
    "parameters",
    "vocabulary",
    "score",
];

const OVERVIEW: &str = "\
stem measures how easy TypeScript code is for a person to follow.

Each function gets four numbers:
  cognitive-complexity  how hard the logic is to follow
  length                lines of code in the function
  parameters            values every caller must pass, in order
  vocabulary            different names the reader must keep track of

Each number falls in a band: low, moderate, high or very high.
The score (0-100) shows how much of your code sits in the higher bands.

More: stem explain <metric>   or   stem explain score
";

const COGNITIVE: &str = "\
Cognitive complexity: how hard the logic is to follow

Counts
  +1  each if, else if, else, ?:, switch, loop and catch
  +1  each chain of && or || (a && b || c counts 2)
  +1  for every level an if, ?:, switch, loop or catch is nested in
  +1  recursion, and labeled break or continue
   0  ?.  ??  await  try  return

Bands for this project
  {bands}

Why it matters
  Every branch is something to keep in mind. Nesting makes the
  reader hold several conditions at once.

To improve
  Return early instead of nesting. Name complex conditions.
  Replace long if/switch chains with a lookup object.
";

const LENGTH: &str = "\
Length: lines of code in the function

Counts
  +1  each line with code on it, including the function's first line
   0  blank lines and lines with only comments
   0  lines whose first code belongs to a nested function; a short
      callback written on its parent's line counts 0 (but weighs 1)

Bands for this project
  {bands}

Why it matters
  A long function does not fit on one screen. The reader must
  scroll and remember earlier parts while reading later ones.

To improve
  Move a section that does one job into a well-named function.
";

const PARAMETERS: &str = "\
Parameters: values every caller must pass, in order

Counts
  +1  each parameter, including a rest parameter (...args)
  +1  each destructured parameter, so ({ a, b }) counts 1
  +1  each constructor parameter property (private a: A)
   0  a TypeScript this: parameter

Bands for this project
  {bands}

Why it matters
  Every caller must know each value and its position. Values of
  the same type are easy to swap by mistake.

To improve
  Group related parameters into one options object.
";

const VOCABULARY: &str = "\
Vocabulary: different names the reader must keep track of

Counts
  +1  each different name in the function: parameters, locals,
      called functions, imports, property names, JSX names
   0  repeats of a name, types, keywords, literals, this, super,
      labels and the function's own name

Bands for this project
  {bands}

Why it matters
  People can hold only a few things in mind at once. Each new
  name is one more thing to remember.

To improve
  Split the function, or group related values into an object.
";

const SCORE: &str = "\
Score: how much of your code is easy to follow, from 0 to 100

How it works
  Every function counts by its length, so long functions weigh more.
  Each metric starts at 100. Each 1% of code that is
    moderate costs 0.5, high costs 2, very high costs 4.
  The overall score is the average of the four metrics.

Grades
  A 85+, B 70+, C 55+, D 25+, E below 25

Changed code
  The same score, counting only functions you added, changed or
  removed. Watch this number after a change.
";

fn range(low: u32, high: u32) -> String {
    if low == high {
        low.to_string()
    } else {
        format!("{low}-{high}")
    }
}

/// `low 0-5, moderate 6-10, high 11-25, very high 26+`.
pub fn bands_line(metric: Metric, thresholds: &Thresholds) -> String {
    let [t1, t2, t3] = thresholds.get(metric);
    format!(
        "low {}, moderate {}, high {}, very high {}+",
        range(0, t1),
        range(t1 + 1, t2),
        range(t2 + 1, t3),
        t3 + 1
    )
}

/// Text of a topic; `None` is the overview.
pub fn explain(topic: Option<&str>, thresholds: &Thresholds) -> Result<String, StemError> {
    let Some(topic) = topic else {
        return Ok(OVERVIEW.to_string());
    };
    if topic == "score" {
        return Ok(SCORE.to_string());
    }
    let Some(metric) = Metric::from_id(topic) else {
        return Err(StemError::UnknownTopic {
            topic: topic.to_string(),
            topics: TOPICS.join(", "),
        });
    };
    let template = match metric {
        Metric::CognitiveComplexity => COGNITIVE,
        Metric::Length => LENGTH,
        Metric::Parameters => PARAMETERS,
        Metric::Vocabulary => VOCABULARY,
    };
    Ok(template.replace("{bands}", &bands_line(metric, thresholds)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_line_should_show_single_values_without_a_range() {
        let line = bands_line(Metric::Parameters, &Thresholds::default());
        assert_eq!(line, "low 0-3, moderate 4, high 5-6, very high 7+");
    }
}
