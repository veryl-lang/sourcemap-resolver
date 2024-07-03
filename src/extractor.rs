use regex::Regex;
use std::collections::VecDeque;
use std::ops::Range;
use std::path::PathBuf;

#[derive(Debug)]
pub struct ExtractResult {
    pub range: Range<u32>,
    pub path: PathBuf,
    pub line: u32,
    pub column: Option<u32>,
}

struct Pattern {
    window: usize,
    regex: Regex,
}

pub struct Extractor {
    patterns: Vec<Pattern>,
    window: usize,
    lines: VecDeque<(String, Option<ExtractResult>)>,
}

impl Extractor {
    pub fn new() -> Self {
        let patterns: Vec<Pattern> = vec![
            // "path", 10
            Pattern {
                window: 1,
                regex: Regex::new(r###""?(?<path>[^ "\n]+)"?, (?:line )?(?<line>[0-9]+)"###)
                    .unwrap(),
            },
            // path:10:10
            Pattern {
                window: 1,
                regex: Regex::new(r"(?<path>[^: \[\]]+):(?<line>[0-9]+)(?::(?<column>[0-9]+))?")
                    .unwrap(),
            },
            // File: path Line: 10
            Pattern {
                window: 1,
                regex: Regex::new(r"File: (?<path>[^: \[\]]+) Line: (?<line>[0-9]+)").unwrap(),
            },
            // line 10 in file
            // 'path'
            Pattern {
                window: 2,
                regex: Regex::new(r"line (?<line>[0-9]+) in file\s*\n\s*'(?<path>[^']+)'").unwrap(),
            },
        ];
        let window = patterns.iter().map(|x| x.window).max().unwrap_or(1);

        Self {
            patterns,
            window,
            lines: VecDeque::new(),
        }
    }

    pub fn push_line(&mut self, line: &str) -> Option<(String, Option<ExtractResult>)> {
        let mut ret = None;

        self.lines.push_back((line.to_string(), None));

        if self.lines.len() > self.window {
            ret = self.lines.pop_front();
        } else if self.lines.len() < self.window {
            return None;
        }

        let text = self
            .lines
            .iter()
            .map(|x| x.0.clone())
            .collect::<Vec<_>>()
            .join("\n");

        for pattern in self.patterns.iter() {
            if let Some(caps) = pattern.regex.captures(&text) {
                let start = caps.get(0).unwrap().start();
                let end = caps.get(0).unwrap().end();
                let path = caps.name("path").unwrap().as_str().to_string();
                let line = caps.name("line").unwrap().as_str().parse::<u32>().unwrap();
                let column = caps
                    .name("column")
                    .map(|x| x.as_str().parse::<u32>().unwrap());

                let mut line_start = 0;
                let mut line_end = 0;
                for (text, extract) in &mut self.lines {
                    line_end += text.len() + 1;

                    if line_start <= end && end < line_end {
                        let start = start.saturating_sub(line_start) as u32;
                        let end = (end - line_start) as u32;
                        *extract = Some(ExtractResult {
                            range: Range { start, end },
                            path: PathBuf::from(path.clone()),
                            line,
                            column,
                        });
                    }

                    line_start = line_end;
                }
            }
        }

        ret
    }

    pub fn end(&mut self) -> Vec<(String, Option<ExtractResult>)> {
        self.lines.drain(0..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(text: &str) -> Vec<(String, Option<ExtractResult>)> {
        let mut extractor = Extractor::new();
        let mut ret = Vec::new();
        for line in text.lines() {
            if let Some(x) = extractor.push_line(line) {
                ret.push(x);
            }
        }
        ret.append(&mut extractor.end());
        ret
    }

    #[test]
    fn vcs() {
        let src = r##"
test.sv, 31
"##;

        let ret = extract(&src);
        assert_eq!(ret.len(), 2);
        assert_eq!(ret[1].0, "test.sv, 31");
        assert_eq!(ret[1].1.as_ref().unwrap().range, 0..11);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 31);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);

        let src = r##"
"test.sv", 10: test1.unnamed$$_1: started at 0s failed at 0s
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 2);
        assert_eq!(
            ret[1].0,
            "\"test.sv\", 10: test1.unnamed$$_1: started at 0s failed at 0s"
        );
        assert_eq!(ret[1].1.as_ref().unwrap().range, 0..13);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 10);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);

        let src = r##"
$finish called from file "test.sv", line 12.
"##;

        let ret = extract(&src);
        assert_eq!(ret.len(), 2);
        assert_eq!(ret[1].0, "$finish called from file \"test.sv\", line 12.");
        assert_eq!(ret[1].1.as_ref().unwrap().range, 25..43);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 12);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);
    }

    #[test]
    fn vivado() {
        let src = r##"
Time: 0 ps  Iteration: 0  Process: /test1/Initial7_0  Scope: test1  File: test.sv Line: 9
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 2);
        assert_eq!(
            ret[1].0,
            "Time: 0 ps  Iteration: 0  Process: /test1/Initial7_0  Scope: test1  File: test.sv Line: 9"
        );
        assert_eq!(ret[1].1.as_ref().unwrap().range, 68..89);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 9);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);

        let src = r##"
ERROR: [VRFC 10-4982] syntax error near 'endmodule' [test.sv:23]
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 2);
        assert_eq!(
            ret[1].0,
            "ERROR: [VRFC 10-4982] syntax error near 'endmodule' [test.sv:23]"
        );
        assert_eq!(ret[1].1.as_ref().unwrap().range, 53..63);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 23);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);
    }

    #[test]
    fn verilator() {
        let src = r##"
%Error: test.sv:11: Verilog $stop
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 2);
        assert_eq!(ret[1].0, "%Error: test.sv:11: Verilog $stop");
        assert_eq!(ret[1].1.as_ref().unwrap().range, 8..18);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 11);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);

        let src = r##"
%Error: test.sv:23:1: syntax error, unexpected endmodule
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 2);
        assert_eq!(
            ret[1].0,
            "%Error: test.sv:23:1: syntax error, unexpected endmodule"
        );
        assert_eq!(ret[1].1.as_ref().unwrap().range, 8..20);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 23);
        assert_eq!(ret[1].1.as_ref().unwrap().column, Some(1));
    }

    #[test]
    fn design_compiler() {
        let src = r##"
Inferred memory devices in process
	in routine ModuleA line 10 in file
		'test.sv'.
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 4);
        assert_eq!(ret[3].0, "\t\t'test.sv'.");
        assert_eq!(ret[3].1.as_ref().unwrap().range, 0..11);
        assert_eq!(ret[3].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[3].1.as_ref().unwrap().line, 10);
        assert_eq!(ret[3].1.as_ref().unwrap().column, None);

        let src = r##"
Warning:  test.sv:67: DEFAULT branch of CASE statement cannot be reached. (ELAB-311)
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 2);
        assert_eq!(
            ret[1].0,
            "Warning:  test.sv:67: DEFAULT branch of CASE statement cannot be reached. (ELAB-311)"
        );
        assert_eq!(ret[1].1.as_ref().unwrap().range, 10..20);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 67);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);
    }

    #[test]
    fn formality() {
        let src = r##"
Warning: INITIAL statements are not supported. (File: test.sv Line: 22)  (FMR_VLOG-101)
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 2);
        assert_eq!(
            ret[1].0,
            "Warning: INITIAL statements are not supported. (File: test.sv Line: 22)  (FMR_VLOG-101)"
        );
        assert_eq!(ret[1].1.as_ref().unwrap().range, 48..70);
        assert_eq!(ret[1].1.as_ref().unwrap().path.to_string_lossy(), "test.sv");
        assert_eq!(ret[1].1.as_ref().unwrap().line, 22);
        assert_eq!(ret[1].1.as_ref().unwrap().column, None);
    }
}
