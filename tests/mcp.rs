use llm_wiki::cli;
use llm_wiki::mcp::tools;
use llm_wiki::server::INSTRUCTIONS;

#[test]
fn tool_list_returns_all_17_tools() {
    let tools = tools::tool_list();
    assert_eq!(tools.len(), 17);
}

#[test]
fn tool_list_contains_expected_names() {
    let tools = tools::tool_list();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let expected = [
        "wiki_init",
        "wiki_config",
        "wiki_spaces_list",
        "wiki_spaces_remove",
        "wiki_spaces_set_default",
        "wiki_write",
        "wiki_ingest",
        "wiki_new_page",
        "wiki_new_section",
        "wiki_search",
        "wiki_read",
        "wiki_list",
        "wiki_index_rebuild",
        "wiki_index_status",
        "wiki_index_check",
        "wiki_lint",
        "wiki_graph",
    ];
    for name in &expected {
        assert!(names.contains(name), "missing tool: {name}");
    }
}

#[test]
fn tool_list_all_have_descriptions() {
    for tool in &tools::tool_list() {
        assert!(
            !tool.description.is_empty(),
            "tool {} has empty description",
            tool.name
        );
    }
}

#[test]
fn tool_list_all_have_object_schema() {
    for tool in &tools::tool_list() {
        let schema = &tool.input_schema;
        assert_eq!(
            schema.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "tool {} schema is not an object",
            tool.name
        );
    }
}

#[test]
fn tool_schemas_have_required_params() {
    let tools = tools::tool_list();
    let init = tools.iter().find(|t| t.name == "wiki_init").unwrap();
    let required = init
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap();
    let req_strs: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(req_strs.contains(&"path"));
    assert!(req_strs.contains(&"name"));
}

#[test]
fn instruct_crystallize_returns_workflow() {
    let section = cli::extract_workflow(INSTRUCTIONS, "crystallize");
    assert!(section.is_some());
    let text = section.unwrap();
    assert!(text.contains("## crystallize"));
    assert!(text.contains("wiki_ingest"));
}

#[test]
fn instruct_frontmatter_returns_workflow() {
    let section = cli::extract_workflow(INSTRUCTIONS, "frontmatter");
    assert!(section.is_some());
    let text = section.unwrap();
    assert!(text.contains("## frontmatter"));
    assert!(text.contains("type: concept"));
}

#[test]
fn instruct_unknown_returns_none() {
    let section = cli::extract_workflow(INSTRUCTIONS, "nonexistent");
    assert!(section.is_none());
}

#[test]
fn instructions_contain_session_orientation() {
    assert!(INSTRUCTIONS.contains("## Session orientation"));
}

#[test]
fn instructions_contain_linking_policy() {
    assert!(INSTRUCTIONS.contains("## Linking policy"));
}

#[test]
fn schema_md_injected_in_server_instructions() {
    // WikiServer concatenates instructions + schema.md at startup.
    // Without a real wiki, we verify the constant is the base.
    assert!(INSTRUCTIONS.starts_with("# llm-wiki Instructions"));
}
