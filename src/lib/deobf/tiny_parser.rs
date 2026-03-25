use std::collections::HashMap;
use nom::branch::alt;
use nom::Parser;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::*;
use nom::combinator::{all_consuming, map, opt, value};
use nom::IResult;
use nom::multi::fold_many0;
use nom::sequence::{delimited, preceded, separated_pair};

/// A minimal parser for the Tiny v2 mapping format (https://wiki.fabricmc.net/documentation:tiny2)
/// Only cares about classes and methods

pub struct Mappings {
    pub classes: HashMap<String, String>,
    pub methods: HashMap<String, String>
}
impl Mappings {
    fn new() -> Self {
        Self {
            classes: HashMap::new(),
            methods: HashMap::new(),
        }
    }
}

#[derive(Debug)]
struct ClassMapping {
    orig: String,
    deobf: String,
    subsections: ClassSubsections
}

#[derive(Debug)]
struct ClassSubsections {
    methods: HashMap<String, String>,
}
impl ClassSubsections {
    fn new() -> Self {
        Self {
            methods: HashMap::new(),
        }
    }
}

fn safe_string(input: &str) -> IResult<&str, &str> {
    is_not("\\\r\n\t\0").parse(input)
}

fn parse_property(input: &str) -> IResult<&str, ()> {
    value((), (tab, is_not("\r\n"), line_ending)).parse(input)
}

fn parse_properties(input: &str) -> IResult<&str, ()> {
    fold_many0(parse_property, || (), |_, _| ()).parse(input)
}

fn parse_header(input: &str) -> IResult<&str, ()> {
    value((), (tag("tiny\t2"), is_not("\r\n"), line_ending, parse_properties)).parse(input)
}

fn parse_class_comment(input: &str) -> IResult<&str, Option<(&str, &str)>> {
    value(None, (tag("\tc"), is_not("\r\n"), opt(line_ending))).parse(input)
}

fn parse_member_comment(input: &str) -> IResult<&str, ()> {
    value((), (tag("\t\tc"), is_not("\r\n"), opt(line_ending))).parse(input)
}

fn parse_var_comment(input: &str) -> IResult<&str, ()> {
    value((), (tag("\t\t\tc"), is_not("\r\n"), opt(line_ending))).parse(input)
}

fn parse_class_header(input: &str) -> IResult<&str, (&str, &str)> {
    delimited(tag("c\t"), separated_pair(safe_string, tab, safe_string), opt(line_ending)).parse(input)
}

fn parse_field_subsections(input: &str) -> IResult<&str, ()> {
    fold_many0(parse_member_comment, || (), |_, _| ()).parse(input)
}

fn parse_field(input: &str) -> IResult<&str, Option<(&str, &str)>> {
    value(None, (tag("\tf"), is_not("\r\n"), opt(line_ending), parse_field_subsections)).parse(input)
}

fn parse_method_parameter(input: &str) -> IResult<&str, ()> {
    value((), (tag("\t\tp"), is_not("\r\n"), opt(line_ending), parse_method_parameter_subsections)).parse(input)
}

fn parse_method_variable(input: &str) -> IResult<&str, ()> {
    value((), (tag("\t\tv"), is_not("\r\n"), opt(line_ending), parse_method_parameter_subsections)).parse(input)
}

fn parse_method_parameter_subsections(input: &str) -> IResult<&str, ()> {
    value((), fold_many0(parse_var_comment, || (), |_, _| ())).parse(input)
}

fn parse_method_subsections(input: &str) -> IResult<&str, ()> {
    fold_many0(alt((parse_member_comment, parse_method_parameter, parse_method_variable)), || (), |_, _| ()).parse(input)
}

fn parse_method(input: &str) -> IResult<&str, Option<(&str, &str)>> {
    map(delimited(
        (tag("\tm\t"), safe_string, tab),
        separated_pair(safe_string, tab, safe_string),
        (opt(is_not("\r\n")), opt(line_ending), parse_method_subsections)
    ), |(name, desc)| Some((name, desc))).parse(input)
}

fn parse_class_subsections(input: &str) -> IResult<&str, ClassSubsections> {
    fold_many0(alt((parse_class_comment, parse_field, parse_method)), ClassSubsections::new, |mut acc, item| {
        if let Some(item) = item {
            acc.methods.insert(String::from(item.0), String::from(item.1));
        }
        acc
    }).parse(input)
}

fn parse_class(input: &str) -> IResult<&str, ClassMapping> {
    map((parse_class_header, parse_class_subsections), |(header, class_subsections)| {
        ClassMapping {
            orig: header.0.to_string(),
            deobf: header.1.to_string(),
            subsections: class_subsections,
        }
    }).parse(input)
}

pub fn parse_tiny(input: &str) -> IResult<&str, Mappings> {
    all_consuming(preceded(parse_header, fold_many0(parse_class, Mappings::new, |mut acc, class| {
        acc.classes.insert(class.orig.clone(), class.deobf.clone());
        acc.methods.extend(class.subsections.methods);
        acc
    }))).parse(input)
}

#[cfg(test)]
mod tests {
    use nom::Finish;
    use crate::deobf::tiny_parser::{parse_class, parse_class_header, parse_class_subsections, parse_field, parse_header, parse_method, parse_tiny};

    #[test]
    fn test_parse_header() {
        let header_lf = "tiny\t2\t0\tintermediary\tnamed\nc";
        let header_crlf = "tiny\t2\t0\tintermediary\tnamed\r\nc";

        let header_lf_str = String::from(header_lf);

        assert_eq!(parse_header(&header_lf_str), Ok(("c", ())));
        assert_eq!(parse_header(header_crlf), Ok(("c", ())));
    }

    #[test]
    fn test_parse_header_properties() {
        let header_properties = "tiny\t2\t0\tintermediary\tnamed\r\n\tprop1\tval1\r\n\tprop2\tval2\r\nc";
        assert_eq!(parse_header(header_properties), Ok(("c", ())));
    }

    #[test]
    fn test_parse_class_header() {
        let class_header = "c\tnet/minecraft/world/level/chunk/LevelChunk\tnet/minecraft/world/level/chunk/LevelChunk";
        assert_eq!(parse_class_header(class_header), Ok(("", ("net/minecraft/world/level/chunk/LevelChunk", "net/minecraft/world/level/chunk/LevelChunk"))));
    }

    #[test]
    fn test_parse_field() {
        let field = "\tf\tI\tfield1\tfield1";
        let field_with_comments = "\tf\tI\tfield1\tfield1\n\t\tc\tHello world\n\t\tc\tPoop\n";
        assert_eq!(parse_field(field), Ok(("", None)));
        assert_eq!(parse_field(field_with_comments), Ok(("", None)));
    }

    #[test]
    fn test_parse_method() {
        let method = "\tm\t()Lnet/minecraft/unmapped/C_3674802;\tm_6501356\tgetForcedSpawnPoint";
        let method_with_parameters = "\tm\t()Lnet/minecraft/unmapped/C_3674802;\tm_6501356\tgetForcedSpawnPoint\n\t\tp\t1\t\ttimeOfDay\n\t\tp\t2\t\ttickDelta";
        assert_eq!(parse_method(method), Ok(("", Some(("m_6501356", "getForcedSpawnPoint")))));
        assert_eq!(parse_method(method_with_parameters), Ok(("", Some(("m_6501356", "getForcedSpawnPoint")))));
    }

    #[test]
    fn test_parse_class_subsections(){
        let subsections = "\tm\t()V\tm_0311145\ttick";
        let subsections = parse_class_subsections(subsections).finish().unwrap().1;
        assert_eq!(subsections.methods.len(), 1);
        assert_eq!(subsections.methods.get("m_0311145").unwrap(), "tick")
    }

    #[test]
    fn test_parse_class() {
        let class = "c\tnet/minecraft/unmapped/C_0005350\tnet/minecraft/client/render/Tickable\n\tm\t()V\tm_0311145\ttick";
        let class_mapping = parse_class(class).finish().unwrap().1;
        assert_eq!(class_mapping.orig, "net/minecraft/unmapped/C_0005350");
        assert_eq!(class_mapping.deobf, "net/minecraft/client/render/Tickable");
        assert_eq!(class_mapping.subsections.methods.len(), 1);
        assert_eq!(class_mapping.subsections.methods.get("m_0311145").unwrap(), "tick");
    }

    #[test]
    fn test_parse_tiny() {
        let tiny = r"tiny	2	0	intermediary	named
c	net/minecraft/client/ClientBrandRetriever	net/minecraft/client/ClientBrandRetriever
	m	()Ljava/lang/String;	getClientModName	getClientModName
c	net/minecraft/client/main/Main	net/minecraft/client/main/Main
	m	(Ljava/lang/String;)Z	m_0158628	isNotNullOrEmpty
		p	0		s
	m	([Ljava/lang/String;)V	main	main
		p	0		args";
        let class_mapping = parse_tiny(tiny).finish().unwrap().1;
        assert_eq!(class_mapping.classes.len(), 2);
        assert_eq!(class_mapping.methods.len(), 3);
        assert_eq!(class_mapping.classes.get("net/minecraft/client/ClientBrandRetriever").unwrap(), "net/minecraft/client/ClientBrandRetriever");
        assert_eq!(class_mapping.classes.get("net/minecraft/client/main/Main").unwrap(), "net/minecraft/client/main/Main");
        assert_eq!(class_mapping.methods.get("getClientModName").unwrap(), "getClientModName");
        assert_eq!(class_mapping.methods.get("m_0158628").unwrap(), "isNotNullOrEmpty");
        assert_eq!(class_mapping.methods.get("main").unwrap(), "main");
    }

    #[test]
    fn test_parse_full_tiny() {
        let tiny = include_str!("mappings.tiny");
        let class_mapping = parse_tiny(tiny).finish().unwrap().1;
        assert_eq!(class_mapping.classes.len(), 2541);
        assert_eq!(class_mapping.classes["net/minecraft/unmapped/C_3755722"], "net/minecraft/item/Item");
    }
}