use once_cell::sync::Lazy;
use regex::Regex;
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
    window: u32,
    regex: Regex,
}

pub fn extract(src: &str) -> Vec<ExtractResult> {
    static PATTERNS: Lazy<Vec<Pattern>> = Lazy::new(|| {
        vec![
            // "path", 10
            Pattern {
                window: 1,
                regex: Regex::new(r###""?(?<path>[^ "\n]+)"?, (?<line>[0-9]+)"###).unwrap(),
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
        ]
    });

    let max_window = PATTERNS.iter().map(|x| x.window).max().unwrap_or(1);

    let mut beg = 0;
    let mut end = 0;

    let mut ret = Vec::new();

    while end != src.len() {
        for _ in 0..max_window {
            if let Some(x) = src[end..].find('\n') {
                end += x + 1;
            } else {
                end = src.len();
            }
        }

        for pattern in PATTERNS.iter() {
            if let Some(caps) = pattern.regex.captures(&src[beg..end]) {
                let start = caps.get(0).unwrap().start() as u32 + beg as u32;
                let end = caps.get(0).unwrap().end() as u32 + beg as u32;
                let path = caps.name("path").unwrap().as_str().to_string();
                let line = caps.name("line").unwrap().as_str().parse::<u32>().unwrap();
                let column = caps
                    .name("column")
                    .map(|x| x.as_str().parse::<u32>().unwrap());

                ret.push(ExtractResult {
                    range: Range { start, end },
                    path: PathBuf::from(path),
                    line,
                    column,
                });
            }
        }

        beg = end;
    }

    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vcs() {
        let src = r##"
test.sv, 31
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 1..12);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 31);
        assert_eq!(ret[0].column, None);

        let src = r##"
"test.sv", 10: test1.unnamed$$_1: started at 0s failed at 0s
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 1..14);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 10);
        assert_eq!(ret[0].column, None);
    }

    #[test]
    fn vivado() {
        let src = r##"
Time: 0 ps  Iteration: 0  Process: /test1/Initial7_0  Scope: test1  File: test.sv Line: 9
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 69..90);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 9);
        assert_eq!(ret[0].column, None);

        let src = r##"
ERROR: [VRFC 10-4982] syntax error near 'endmodule' [test.sv:23]
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 54..64);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 23);
        assert_eq!(ret[0].column, None);
    }

    #[test]
    fn verilator() {
        let src = r##"
%Error: test.sv:11: Verilog $stop
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 9..19);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 11);
        assert_eq!(ret[0].column, None);

        let src = r##"
%Error: test.sv:23:1: syntax error, unexpected endmodule
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 9..21);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 23);
        assert_eq!(ret[0].column, Some(1));
    }

    #[test]
    fn design_compiler() {
        let src = r##"
Inferred memory devices in process
	in routine ModuleA line 10 in file
		'test.sv'.
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 56..83);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 10);
        assert_eq!(ret[0].column, None);

        let src = r##"
Warning:  test.sv:67: DEFAULT branch of CASE statement cannot be reached. (ELAB-311)
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 11..21);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 67);
        assert_eq!(ret[0].column, None);
    }

    #[test]
    fn formality() {
        let src = r##"
Warning: INITIAL statements are not supported. (File: test.sv Line: 22)  (FMR_VLOG-101)
"##;

        let ret = extract(src);
        assert_eq!(ret.len(), 1);
        assert_eq!(ret[0].range, 49..71);
        assert_eq!(ret[0].path.to_string_lossy(), "test.sv");
        assert_eq!(ret[0].line, 22);
        assert_eq!(ret[0].column, None);
    }
}
