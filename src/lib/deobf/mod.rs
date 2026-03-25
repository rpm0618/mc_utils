use std::sync::LazyLock;
use nom::Finish;
use crate::deobf::tiny_parser::{Mappings};

pub mod tiny_parser;

const TINY_SOURCE: &str = include_str!("mappings.tiny");
pub static MAPPINGS: LazyLock<Mappings> = LazyLock::new(|| {
    tiny_parser::parse_tiny(TINY_SOURCE).finish().unwrap().1
});

pub fn deobfuscate_method(method: &str) -> Option<String> {
    let Some((class, method)) = method.rsplit_once(".") else {
        return None;
    };

    let path = class.replace(".", "/");

    let Some(deobf_class) = MAPPINGS.classes.get(&path) else {
        return None;
    };

    let Some(deobf_method) = MAPPINGS.methods.get(method) else {
        return None;
    };

    Some(deobf_class.replace("/", ".") + "." + deobf_method)
}

#[cfg(test)]
mod tests {
    use crate::deobf::deobfuscate_method;

    #[test]
    fn test_deobf_method() {
        let method = "net.minecraft.unmapped.C_0987157.m_4129669";
        assert_eq!(deobfuscate_method(method), Some("net.minecraft.world.gen.chunk.OverworldChunkGenerator.populateChunk".to_string()));
    }
}