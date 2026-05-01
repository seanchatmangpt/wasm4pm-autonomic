//! MCP JSON-RPC Transport: Transport layer for Model Context Protocol communication.
//!
//! Provides the JSON-RPC 2.0 framing and serialization logic for invoking
//! external tools and harvesting results into the provenance chain.

use crate::construct8::Triple;
use crate::runtime::mcp::MCPCall;
use crate::runtime::mcp_result::MCPResult;
use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 Request structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0").
    pub jsonrpc: String,
    /// Method name to invoke.
    pub method: String,
    /// Parameters for the method.
    pub params: serde_json::Value,
    /// Request identifier.
    pub id: u64,
}

/// JSON-RPC 2.0 Response structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (must be "2.0").
    pub jsonrpc: String,
    /// Successful result, if any.
    pub result: Option<serde_json::Value>,
    /// Error information, if any.
    pub error: Option<JsonRpcError>,
    /// Response identifier matching the request.
    pub id: u64,
}

/// JSON-RPC 2.0 Error structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,
    /// Human-readable error message.
    pub message: String,
    /// Optional structured error data.
    pub data: Option<serde_json::Value>,
}

/// Transport for MCP tool calls.
///
/// Handles the projection of internal tool calls into standard JSON-RPC 2.0
/// requests and parses the resulting responses.
#[derive(Debug, Default)]
pub struct MCPTransport;

impl MCPTransport {
    /// Maps an [`MCPCall`] to a [`JsonRpcRequest`].
    ///
    /// Translates internal `ToolId` and `MCPArguments` into the JSON-RPC
    /// format expected by MCP servers.
    pub fn map_call(&self, call: &MCPCall, id: u64) -> JsonRpcRequest {
        // Map ToolId to a canonical tool name string.
        let tool_name = match call.tool_id.0 {
            1 => "mcp:tool:ask_evidence",
            2 => "mcp:tool:resolve_phrase",
            3 => "mcp:tool:validate_transition",
            4 => "mcp:tool:emit_receipt",
            42 => "mcp:tool:test_tool",
            1001 => "mcp:tool:get_package_status",
            _ => "mcp:tool:unknown",
        };

        JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": tool_name,
                "arguments": {
                    "param0": call.arguments.param0,
                    "param1": call.arguments.param1,
                    "required_vars": call.required_vars,
                }
            }),
            id,
        }
    }

    /// Parses a [`JsonRpcResponse`] into an [`MCPResult`].
    ///
    /// Extracts the triple content from the tool response and validates the schema.
    ///
    /// # Errors
    ///
    /// Returns a descriptive error string if the response contains a JSON-RPC error
    /// or if the result format is invalid.
    pub fn parse_response(&self, response: &JsonRpcResponse) -> Result<MCPResult, String> {
        if let Some(err) = &response.error {
            return Err(format!("JSON-RPC Error ({}): {}", err.code, err.message));
        }

        let result = response
            .result
            .as_ref()
            .ok_or_else(|| "Missing result in response".to_string())?;

        // MCP tool calls usually return { "content": [...] }
        // We look for a custom "triples" field to populate MCPResult.
        let triples_val = result
            .get("triples")
            .ok_or_else(|| "Missing 'triples' in result".to_string())?;

        let triples: Vec<Triple> = serde_json::from_value(triples_val.clone())
            .map_err(|e| format!("Invalid triple format: {}", e))?;

        Ok(MCPResult { triples })
    }

    /// Performs a synchronous simulated call for verification.
    ///
    /// This provides a verifiable path for tool execution without requiring
    /// external network dependencies in the build environment.
    pub fn simulate_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = if request.method == "tools/call" {
            // Produce a verifiable response for the requested tool.
            Some(serde_json::json!({
                "content": [
                    { "type": "text", "text": "Tool executed successfully." }
                ],
                "triples": [
                    { "subject": 0x100, "predicate": 0x10, "object": 0x200 }
                ]
            }))
        } else {
            None
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            result,
            error: None,
            id: request.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiled::CompiledFieldSnapshot;
    use crate::compiled_hook::Predicate;
    use crate::field::FieldContext;
    use crate::multimodal::{ContextBundle, PostureBundle};
    use crate::packs::TierMasks;
    use crate::runtime::mcp::{MCPArguments, MCPCall, ToolId};
    use crate::runtime::mcp_guard::{GuardRejection, StripsGuard};
    use crate::runtime::mcp_result::{DefaultProjector, ResultProjector};
    use crate::runtime::ClosedFieldContext;

    #[test]
    fn test_mcp_transport_full_loop() {
        let transport = MCPTransport;

        // 1. Setup a tool call
        let call = MCPCall {
            tool_id: ToolId(42),
            required_vars: 1u64 << Predicate::HAS_RDF_TYPE,
            arguments: MCPArguments::default(),
            collapse_fn: crate::ids::CollapseFn::ExpertRule,
        };

        // 2. Setup context that fulfills the precondition
        let mut field = FieldContext::new("test");
        field.load_field_state(
            "<http://example.org/s> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/t> .\n"
        ).unwrap();
        let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: PostureBundle::default(),
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };

        // 3. Verify StripsGuard ADMITS the call
        let guard_result = StripsGuard::evaluate(&call, &context);
        assert!(
            guard_result.is_ok(),
            "Guard should admit call when precondition is met"
        );

        // 4. Map to JSON-RPC request
        let request = transport.map_call(&call, 1);
        assert_eq!(request.method, "tools/call");
        assert_eq!(request.params["name"], "mcp:tool:test_tool");

        // 5. Simulate send/receive
        let response = transport.simulate_call(request);
        assert!(response.result.is_some());

        // 6. Parse response to MCPResult
        let mcp_result = transport.parse_response(&response).unwrap();
        assert_eq!(mcp_result.triples.len(), 1);

        // 7. Project to Construct8
        let projector = DefaultProjector;
        let construct = projector.project(&mcp_result).unwrap();
        assert_eq!(construct.len(), 1);

        let triple = construct.iter().next().unwrap();
        assert_eq!(triple.subject.0, 0x100);
        assert_eq!(triple.predicate.0, 0x10);
        assert_eq!(triple.object.0, 0x200);
    }

    #[test]
    fn test_strips_guard_blocks_unfulfilled_precondition() {
        let call = MCPCall {
            tool_id: ToolId(42),
            collapse_fn: crate::ids::CollapseFn::Grounding,
            required_vars: 1u64 << Predicate::DD_PRESENT,
            arguments: MCPArguments::default(),
        };

        // Empty context (no DigitalDocuments)
        let field = FieldContext::new("test");
        let snap = CompiledFieldSnapshot::from_field(&field).unwrap();
        let context = ClosedFieldContext {
            snapshot: std::sync::Arc::new(snap.clone()),
            posture: PostureBundle::default(),
            context: ContextBundle::default(),
            tiers: TierMasks::ZERO,
            human_burden: 0,
        };

        let guard_result = StripsGuard::evaluate(&call, &context);
        assert_eq!(guard_result, Err(GuardRejection::MissingPrecondition));
    }
}
