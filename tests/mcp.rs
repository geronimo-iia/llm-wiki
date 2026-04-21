use llm_wiki::mcp::tools;

#[test]
fn tool_list_returns_16_tools() {
    let tools = tools::tool_list();
    assert_eq!(tools.len(), 16);
}

#[test]
fn tool_list_contains_expected_names() {
    let tools = tools::tool_list();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let expected = [
        "wiki_spaces_create",
        "wiki_spaces_list",
        "wiki_spaces_remove",
        "wiki_spaces_set_default",
        "wiki_config",
        "wiki_content_read",
        "wiki_content_write",
        "wiki_content_new",
        "wiki_content_commit",
        "wiki_search",
        "wiki_list",
        "wiki_ingest",
        "wiki_index_rebuild",
        "wiki_index_status",
        "wiki_graph",
    ];
    for name in &expected {
        assert!(names.contains(name), "missing tool: {name}");
    }
}

#[test]
fn tool_list_no_removed_tools() {
    let tools = tools::tool_list();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let removed = [
        "wiki_init",
        "wiki_read",
        "wiki_write",
        "wiki_new_page",
        "wiki_new_section",
        "wiki_commit",
        "wiki_lint",
        "wiki_index_check",
    ];
    for name in &removed {
        assert!(!names.contains(name), "tool should be removed: {name}");
    }
}

#[test]
fn tool_list_all_have_descriptions() {
    for tool in &tools::tool_list() {
        assert!(
            !tool.description.as_ref().is_none_or(|d| d.is_empty()),
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
fn spaces_create_requires_path_and_name() {
    let tools = tools::tool_list();
    let tool = tools
        .iter()
        .find(|t| t.name == "wiki_spaces_create")
        .unwrap();
    let required = tool
        .input_schema
        .get("required")
        .unwrap()
        .as_array()
        .unwrap();
    let req: Vec<&str> = required.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(req.contains(&"path"));
    assert!(req.contains(&"name"));
}

#[test]
fn content_new_has_section_and_name_and_type_params() {
    let tools = tools::tool_list();
    let tool = tools.iter().find(|t| t.name == "wiki_content_new").unwrap();
    let props = tool
        .input_schema
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(props.contains_key("section"), "missing section param");
    assert!(props.contains_key("name"), "missing name param");
    assert!(props.contains_key("type"), "missing type param");
    assert!(props.contains_key("bundle"), "missing bundle param");
}

#[test]
fn search_has_type_param() {
    let tools = tools::tool_list();
    let tool = tools.iter().find(|t| t.name == "wiki_search").unwrap();
    let props = tool
        .input_schema
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(props.contains_key("type"), "missing type param");
}

#[test]
fn graph_has_relation_param() {
    let tools = tools::tool_list();
    let tool = tools.iter().find(|t| t.name == "wiki_graph").unwrap();
    let props = tool
        .input_schema
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(props.contains_key("relation"), "missing relation param");
}
