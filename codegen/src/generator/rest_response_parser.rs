use inflector::Inflector;
use botocore::{Service, Operation, Shape, ShapeType, Member};


pub fn generate_response_headers_parser(service: &Service,
                                        operation: &Operation)
                                        -> Option<String> {
    // nothing to do if there's no output type
    if operation.output.is_none() {
        return None;
    }

    let shape = &service.shapes[&operation.output.as_ref().unwrap().shape];
    let members = shape.members.as_ref().unwrap();

    let parser_pieces = members.iter()
        .filter_map(|(member_name, member)| {
            match member.location.as_ref().map(String::as_ref) {
                Some("header") => Some(parse_single_header(service, shape, member_name, member)),
                Some("headers") => {
                    Some(parse_multiple_headers(service, shape, member_name, member))
                }
                _ => None,
            }
        })
        .collect::<Vec<String>>();

    if !parser_pieces.is_empty() {
        Some(parser_pieces.join("\n"))
    } else {
        None
    }
}

fn parse_multiple_headers(service: &Service,
                          shape: &Shape,
                          member_name: &str,
                          member: &Member)
                          -> String {
    let member_shape = &service.shapes[&member.shape];
    let required = shape.required(member_name);
    match member_shape.shape_type {
        ShapeType::Map => parse_headers_map(member_name, member, required),
        ShapeType::List => parse_headers_list(member_name, member, required),
        shape_type @ _ => {
            panic!("Unknown header shape type {:?} for {}",
                   shape_type,
                   member_name)
        }
    }
}

fn parse_headers_list(member_name: &str, member: &Member, required: bool) -> String {
    let set_statement = if required {
        format!("result.{} = values;", member_name.to_snake_case())
    } else {
        format!("result.{} = Some(values);", member_name.to_snake_case())
    };

    format!("let mut values = Vec::new();
            for (key, value) in response.headers.iter() {{
              if key == \"{location_name}\" {{
                values.push(value);
              }}
            }}
            {set_statement}",
            location_name = member.location_name.as_ref().unwrap(),
            set_statement = set_statement)
}

fn parse_headers_map(member_name: &str, member: &Member, required: bool) -> String {
    let set_statement = if required {
        format!("result.{} = values;", member_name.to_snake_case())
    } else {
        format!("result.{} = Some(values);", member_name.to_snake_case())
    };

    format!("let mut values = ::std::collections::HashMap::new();
    for (key, value) in response.headers.iter() {{
        if key.starts_with(\"{location_name}\") {{
            values.insert(key.replace(\"{location_name}\",\"\"), value.to_owned());
        }}
    }}
    {set_statement}",
            location_name = member.location_name.as_ref().unwrap(),
            set_statement = set_statement)
}

fn parse_single_header(service: &Service,
                       shape: &Shape,
                       member_name: &str,
                       member: &Member)
                       -> String {
    let member_shape = &service.shapes[&member.shape];
    if shape.required(member_name) {
        format!("let value = response.headers.get(\"{location_name}\").unwrap().to_owned();
                 result.{field_name} = {primitive_parser};",
                location_name = member.location_name.as_ref().unwrap(),
                field_name = member_name.to_snake_case(),
                primitive_parser = generate_header_primitive_parser(&member_shape))
    } else {
        format!("if let Some({field_name}) = response.headers.get(\"{location_name}\") {{
                    let value = {field_name}.to_owned();
                    result.{field_name} = Some({primitive_parser})
                  }};",
                location_name = member.location_name.as_ref().unwrap(),
                field_name = member_name.to_snake_case(),
                primitive_parser = generate_header_primitive_parser(&member_shape))
    }
}

fn generate_header_primitive_parser(shape: &Shape) -> String {
    let statement = match shape.shape_type {
        ShapeType::String | ShapeType::Timestamp => "value",
        ShapeType::Integer => "i64::from_str(&value).unwrap()",
        ShapeType::Long => "i64::from_str(&value).unwrap()",
        ShapeType::Double => "f64::from_str(&value).unwrap()",
        ShapeType::Float => "f32::from_str(&value).unwrap()",
        ShapeType::Boolean => "bool::from_str(&value).unwrap()",
        _ => panic!("Unknown primitive shape type"),
    };

    statement.to_string()
}
