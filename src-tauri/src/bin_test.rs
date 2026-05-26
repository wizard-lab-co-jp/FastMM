fn main() {
    let markdown = "# Heading\n\nHello **bold** world\n\n- List item";
    let (blocks, _) = fastmm::ast::parse_document_to_blocks(markdown);
    for b in blocks {
        println!("{:?}", b.block_type);
        println!("{:?}", b.ast_content);
    }
}
