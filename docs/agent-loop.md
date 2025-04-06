# Agent Flow Library Design

This document outlines the design and implementation plan for a new `agent_flow` module within the HAL project. This module will implement an agent loop system based on an Environment model and conversation history, creating a flexible and powerful framework for tool-using agents.

## Core Concepts

The `agent_flow` library is built around several key concepts:

1. **Environment**: A structured representation of the world state that the agent can perceive and modify
2. **History**: A record of the conversation between the agent and user, as well as tool invocations
3. **Tools**: Functions that the agent can call to gather information or take actions in the world
4. **Context**: The combined environment and history information sent to the LLM
5. **Agent Loop**: The cycle of receiving input, updating context, getting LLM responses, and executing tools

## Architecture

The `agent_flow` library is organized around the following key components:

```
agent_flow/
├── agent.rs          # AgentExecutor implementation  
├── context/          # Context management
│   ├── mod.rs        # Module entry point
│   ├── manager.rs    # ContextManager for coordinating environment and history
│   ├── environment.rs # Environment implementation
│   └── history.rs    # History implementation
├── tools/            # Tool integration
│   ├── mod.rs        # Module entry point
│   ├── executor.rs   # Tool discovery and execution
│   └── adapter.rs    # Adapters for RMCP tools
├── llm/              # LLM integration
│   ├── mod.rs        # Module entry point
│   ├── completion.rs # LLM completion handling
│   └── parser.rs     # Response parsing
├── events/           # Event system
│   ├── mod.rs        # Module entry point
│   └── emitter.rs    # Event emission
├── config.rs         # Configuration system
├── error.rs          # Error types and handling
└── lib.rs            # Library entry point
```

### Key Components

#### 1. Environment

The Environment represents the external world state that the agent can perceive and manipulate. It serves as a structured memory for the agent.

```rust
/// A representation of the agent's world state
pub struct Environment {
    /// The underlying data store for environment state
    state: serde_json::Value,
    
    /// Schema for validating environment state (optional)
    schema: Option<serde_json::Value>,
    
    /// Maximum tokens to use for environment serialization
    max_tokens: usize,
}

impl Environment {
    /// Create a new environment with initial state
    pub fn new(initial_state: serde_json::Value) -> Self;
    
    /// Update the environment based on a tool result
    pub fn update_from_tool_result(&mut self, tool_name: &str, result: &str) -> Result<(), AgentError>;
    
    /// Get a value from the environment
    pub fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, AgentError>;
    
    /// Set a value in the environment
    pub fn set<T: Serialize>(&mut self, path: &str, value: T) -> Result<(), AgentError>;
    
    /// Serialize the environment to a string for inclusion in prompts
    pub fn to_context(&self, max_tokens: usize) -> String;
}
```

#### 2. History

The History component maintains a record of the conversation, including messages and tool results.

```rust
/// A record of the conversation and tool interactions
pub struct History {
    /// The entries in the history
    entries: Vec<HistoryEntry>,
    
    /// The token counter for measuring entry sizes
    token_counter: Arc<dyn Fn(&str) -> usize + Send + Sync>,
}

/// An entry in the conversation history
pub enum HistoryEntry {
    /// A message from the user to the agent
    UserMessage { content: String },
    
    /// A message from the agent to the user
    AgentMessage { content: String },
    
    /// The agent's internal thinking process
    AgentThinking { content: String },
    
    /// A tool call made by the agent
    ToolCall { 
        name: String, 
        args: serde_json::Value,
        id: String,
    },
    
    /// The result of a tool call
    ToolResult { 
        id: String, 
        result: String,
        tool_name: String,
    },
}

impl History {
    /// Create a new history
    pub fn new(token_counter: Arc<dyn Fn(&str) -> usize + Send + Sync>) -> Self;
    
    /// Add an entry to the history
    pub fn add(&mut self, entry: HistoryEntry);
    
    /// Get the current token count of the history
    pub fn token_count(&self) -> usize;
    
    /// Prune the history to fit within token constraints
    pub fn prune(&mut self, strategy: PruningStrategy, max_tokens: usize);
    
    /// Convert the history to a string for inclusion in prompts
    pub fn to_context(&self, max_tokens: usize) -> String;
    
    /// Summarize a portion of the history
    pub fn summarize(
        &self, 
        start_idx: usize, 
        end_idx: usize,
        summarizer: &dyn Summarizer
    ) -> Result<String, AgentError>;
}
```

#### 3. Context Manager

The ContextManager coordinates between the Environment and History to create optimized prompts for the LLM.

```rust
/// Manages context construction for LLM prompts
pub struct ContextManager {
    /// The agent's environment
    environment: Environment,
    
    /// The conversation history
    history: History,
    
    /// Maximum tokens to use for context
    max_tokens: usize,
    
    /// Allocation of tokens between environment and history
    environment_allocation: f32,
    
    /// The system prompt template
    system_prompt_template: String,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(
        environment: Environment,
        history: History,
        max_tokens: usize,
        system_prompt_template: String,
    ) -> Self;
    
    /// Build a prompt for the LLM
    pub fn build_prompt(&self, current_task: &str) -> String;
    
    /// Update the environment based on a tool result
    pub fn update_environment(&mut self, tool_name: &str, result: &str) -> Result<(), AgentError>;
    
    /// Add an entry to the history
    pub fn add_to_history(&mut self, entry: HistoryEntry);
    
    /// Get the current token count
    pub fn token_count(&self) -> usize;
    
    /// Prune context if needed to stay within token limits
    pub fn prune_if_needed(&mut self) -> Result<(), AgentError>;
}
```

#### 4. Tool Executor

The ToolExecutor manages tool discovery and execution, including integration with RMCP.

```rust
/// Executes tools on behalf of the agent
pub struct ToolExecutor {
    /// The RMCP client for remote tool execution
    client: Option<rmcp::Client>,
    
    /// Local tools that don't require RMCP
    local_tools: HashMap<String, Box<dyn Tool>>,
    
    /// Tool definitions available to the agent
    tool_definitions: Vec<ToolDefinition>,
}

impl ToolExecutor {
    /// Create a new tool executor
    pub fn new() -> Self;
    
    /// Add a local tool
    pub fn add_tool<T: Tool + 'static>(&mut self, tool: T);
    
    /// Connect to an RMCP server
    pub fn connect_rmcp(&mut self, server_url: &str) -> Result<(), AgentError>;
    
    /// List available tools
    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>, AgentError>;
    
    /// Execute a tool
    pub async fn execute_tool(
        &self,
        name: &str,
        args: &serde_json::Value
    ) -> Result<String, AgentError>;
}
```

#### 5. Agent Executor

The AgentExecutor implements the main agent loop.

```rust
/// Executes the agent loop
pub struct AgentExecutor {
    /// The context manager
    context_manager: ContextManager,
    
    /// The tool executor
    tool_executor: ToolExecutor,
    
    /// The LLM client
    llm_client: Box<dyn LlmClient>,
    
    /// The event emitter
    event_emitter: EventEmitter,
    
    /// Configuration for the agent
    config: AgentConfig,
}

impl AgentExecutor {
    /// Create a new agent executor
    pub fn new(
        context_manager: ContextManager,
        tool_executor: ToolExecutor,
        llm_client: Box<dyn LlmClient>,
        config: AgentConfig,
    ) -> Self;
    
    /// Execute the agent loop
    pub async fn execute(
        &mut self,
        task: &str,
        event_sender: Sender<Result<AgentEvent, AgentError>>
    ) -> Result<ExecutionOutcome, AgentError>;
    
    /// Process an LLM completion
    async fn process_completion(
        &mut self,
        completion: String
    ) -> Result<bool, AgentError>;
    
    /// Execute a tool call
    async fn execute_tool_call(
        &mut self,
        call: &ToolCall
    ) -> Result<(), AgentError>;
}
```

#### 6. Event System

The event system provides real-time feedback on agent activities.

```rust
/// Events emitted by the agent during execution
pub enum AgentEvent {
    /// The agent has produced some explanatory text
    Thinking { text: String },
    
    /// The agent is attempting to call a tool
    ToolCallAttempted { 
        name: String,
        args: serde_json::Value,
        id: String,
    },
    
    /// A tool call has completed
    ToolCallCompleted { 
        id: String,
        result: String,
        tool_name: String,
    },
    
    /// An error occurred that doesn't stop execution
    ExecutionError { error: String },
    
    /// The agent task is complete
    Finished { summary: String },
}

/// Emits events during agent execution
pub struct EventEmitter {
    /// The channel sender for events
    sender: Option<tokio::sync::mpsc::Sender<Result<AgentEvent, AgentError>>>,
}

impl EventEmitter {
    /// Create a new event emitter
    pub fn new(
        sender: Option<tokio::sync::mpsc::Sender<Result<AgentEvent, AgentError>>>
    ) -> Self;
    
    /// Emit an event
    pub fn emit(&self, event: AgentEvent) -> Result<(), AgentError>;
}
```

## Implementation Plan

### Phase 1: Core Infrastructure

1. Set up module structure and core interfaces
2. Implement basic error types and result wrappers
3. Create simple versions of Environment and History
4. Implement token counting utilities
5. Add serialization/deserialization for Environment

### Phase 2: Tool Integration

1. Implement ToolExecutor with local tools support
2. Create adapters for RMCP tools
3. Implement tool discovery and execution
4. Add error handling and retry mechanisms
5. Develop basic telemetry for tool execution

### Phase 3: Context Management

1. Implement ContextManager for environment and history coordination
2. Add token allocation algorithms
3. Implement prompt construction with templating
4. Create pruning strategies for history
5. Add support for summarization

### Phase 4: Agent Loop and LLM Integration

1. Implement AgentExecutor with the main execution loop
2. Create LLM client implementations
3. Add response parsing and tool call extraction
4. Implement event emission for execution feedback
5. Add support for multiple termination conditions

### Phase 5: Configuration and Refinement

1. Implement configuration system with validation
2. Add support for different token counting strategies
3. Implement more sophisticated pruning algorithms
4. Add metrics collection and reporting
5. Optimize performance and token efficiency

### Phase 6: Testing and Documentation

1. Implement mock tools and LLMs for testing
2. Write unit tests for core components
3. Create integration tests for full execution
4. Add property-based tests for key algorithms
5. Write comprehensive documentation and examples

## Dependencies

- **tokio**: Async runtime for concurrent operations
- **serde**: Serialization/deserialization for JSON handling
- **rmcp**: Rust MCP client library for tool integration
- **rig**: LLM integration library for completion requests
- **tracing**: Instrumentation for logging and debugging
- **thiserror**: Error handling utilities
- **async_stream**: Utilities for working with async streams
- **config**: Configuration management
- **jsonpointer**: For accessing JSON paths in Environment

## API Examples

### Basic Agent Execution

```rust
// Create components
let env = Environment::new(json!({ "state": "initial" }));
let history = History::new(Arc::new(|s| s.len() / 4)); // Simple token counter
let context_manager = ContextManager::new(env, history, 4000, "You are a helpful assistant.");
let tool_executor = ToolExecutor::new();
let llm_client = RigLlmClient::new("model_name");
let config = AgentConfig::default();

// Create agent executor
let mut executor = AgentExecutor::new(
    context_manager,
    tool_executor,
    Box::new(llm_client),
    config,
);

// Create event channel
let (tx, mut rx) = tokio::sync::mpsc::channel(32);

// Process events
let event_handler = tokio::spawn(async move {
    while let Some(event) = rx.recv().await {
        match event {
            Ok(AgentEvent::Thinking { text }) => println!("Agent thought: {}", text),
            Ok(AgentEvent::ToolCallAttempted { name, args, .. }) => 
                println!("Tool call: {} with args: {}", name, args),
            // Handle other events...
            _ => {}
        }
    }
});

// Execute agent
let result = executor.execute("Perform this task...", tx).await?;
```

### Custom Tool Integration

```rust
// Define a custom tool
struct WeatherTool;

impl Tool for WeatherTool {
    fn name(&self) -> &str {
        "get_weather"
    }
    
    fn description(&self) -> &str {
        "Get the current weather for a location"
    }
    
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The location to get weather for"
                }
            },
            "required": ["location"]
        })
    }
    
    async fn call(&self, args: serde_json::Value) -> Result<String, ToolError> {
        let location = args["location"].as_str()
            .ok_or_else(|| ToolError::InvalidArgument("location must be a string".to_string()))?;
            
        // Implement weather lookup logic
        // ...
        
        Ok(json!({
            "temperature": 72,
            "conditions": "sunny",
            "location": location
        }).to_string())
    }
}

// Add to executor
let mut tool_executor = ToolExecutor::new();
tool_executor.add_tool(WeatherTool);
```

## Success Criteria

The `agent_flow` library will be considered successful if it:

1. **Provides a flexible, modular framework** for building agents with environment awareness
2. **Efficiently manages token usage** to maximize context utilization
3. **Handles tool execution robustly** with good error recovery
4. **Emits detailed events** for monitoring and UI integration
5. **Scales well** with different agent configurations and tool sets
6. **Has comprehensive test coverage** ensuring reliability
7. **Includes clear documentation** and examples demonstrating various use cases


## Strongly Typed Environment Approach

The initial environment design uses a flexible JSON-based approach, but we can enhance type safety and developer experience with a more strongly typed design that balances flexibility with compile-time safety.

### Core Environment Types

```rust
// Core environment data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub working_directory: PathBuf,
    pub permissions: Vec<String>,
    pub resources: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskState {
    pub objective: String,
    pub status: TaskStatus,
    pub steps_completed: u32,
    pub steps_total: u32,
    pub current_step: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBase {
    pub discovered_facts: Vec<String>,
    pub sources: HashMap<String, String>,
    pub key_concepts: Vec<String>,
    
    // Extensible section for dynamic data
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceState {
    pub files: HashMap<String, FileInfo>,
    pub artifacts: Vec<ArtifactInfo>,
    
    // Extensible section for dynamic data
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

// Main Environment struct
pub struct Environment {
    system: SystemState,
    task: TaskState,
    knowledge: KnowledgeBase,
    workspace: WorkspaceState,
    
    // Additional untyped sections for extensibility
    extensions: HashMap<String, serde_json::Value>,
}
```

### Environment Implementation Highlights

The Environment implementation would provide:

1. **Type-safe accessors** for core sections:
   ```rust
   pub fn system(&self) -> &SystemState { &self.system }
   pub fn system_mut(&mut self) -> &mut SystemState { &mut self.system }
   // Similar for task, knowledge, workspace
   ```

2. **Generic extension methods** for flexible data storage:
   ```rust
   pub fn get_extension<T: DeserializeOwned>(&self, name: &str) -> Result<T, AgentError> { ... }
   pub fn set_extension<T: Serialize>(&mut self, name: &str, value: T) -> Result<(), AgentError> { ... }
   ```

3. **Tool result integration** with specialized handlers:
   ```rust
   pub fn update_from_tool_result(&mut self, tool_name: &str, result: &str) -> Result<(), AgentError> {
       match tool_name {
           "directory_tree" => self.update_workspace_from_directory_tree(&result_value),
           "search" => self.update_knowledge_from_search(&result_value),
           // Other specific tool handlers
           _ => self.update_generic(tool_name, &result_value),
       }
   }
   ```

4. **Context serialization** with token limit awareness:
   ```rust
   pub fn to_context(&self, max_tokens: usize) -> String { ... }
   ```

5. **Builder pattern** for easy construction:
   ```rust
   pub fn builder() -> EnvironmentBuilder { EnvironmentBuilder::new() }
   ```

## Example Usage: Coding Agent

This example shows how a coding agent might use the strongly typed environment:

```rust
// Create the environment for a coding agent
let mut env = Environment::builder()
    .system(SystemState {
        working_directory: PathBuf::from("/Users/josh/projects/hal"),
        permissions: vec!["read".to_string(), "write".to_string(), "execute".to_string()],
        resources: HashMap::from([
            ("memory_limit".to_string(), "8GB".to_string()),
            ("timeout".to_string(), "3600s".to_string())
        ]),
    })
    .task(TaskState {
        objective: "Implement Environment struct for agent_flow module".to_string(),
        status: TaskStatus::InProgress,
        steps_completed: 0,
        steps_total: 4,
        current_step: Some("Explore project structure".to_string()),
    })
    .build();

// Agent explores project structure
env.update_from_tool_result("directory_tree", r#"
{
    "tree": [
        "hal/",
        "  ├── src/",
        "  │   ├── agent_flow/",
        "  │   │   └── mod.rs",
        "  │   ├── coder/",
        "  │   │   ├── executor.rs",
        "  │   │   └── session.rs",
        "  │   └── lib.rs",
        "  ├── docs/",
        "  │   └── agent-loop.md",
        "  ├── Cargo.toml"
    ]
}
"#).unwrap();

// Update task progress
env.task_mut().steps_completed += 1;
env.task_mut().current_step = Some("Examine existing code".to_string());

// Store a code design extension with type safety
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CodeDesign {
    structs: Vec<StructDefinition>,
    design_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StructDefinition {
    name: String,
    fields: Vec<FieldDefinition>,
    methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FieldDefinition {
    name: String,
    field_type: String,
    description: String,
}

// Create and add the design to environment
let environment_design = CodeDesign {
    structs: vec![
        StructDefinition {
            name: "Environment".to_string(),
            fields: vec![
                FieldDefinition {
                    name: "state".to_string(),
                    field_type: "serde_json::Value".to_string(),
                    description: "The underlying data store for environment state".to_string(),
                },
                // More fields...
            ],
            methods: vec![
                "update_from_tool_result(&mut self, tool_name: &str, result: &str) -> Result<(), AgentError>".to_string(),
                // More methods...
            ],
        }
    ],
    design_patterns: vec!["Observer pattern for tool result updates".to_string()],
};

env.set_extension("code_design", environment_design).unwrap();

// Later, retrieve with type safety
let design: CodeDesign = env.get_extension("code_design").unwrap();
println!("Designing struct: {}", design.structs[0].name);
```

## Example Usage: Data Analysis Agent

This example demonstrates a data analysis agent using the environment:

```rust
// Define domain-specific types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub name: String,
    pub rows: usize,
    pub columns: Vec<String>,
    pub summary_stats: HashMap<String, serde_json::Value>,
}

// Create the environment
let mut env = Environment::builder()
    .system(SystemState {
        working_directory: PathBuf::from("/Users/josh/data_analysis"),
        permissions: vec!["read".to_string(), "write".to_string()],
        resources: HashMap::from([
            ("memory_limit".to_string(), "16GB".to_string()),
            ("compute_units".to_string(), "10".to_string())
        ]),
    })
    .task(TaskState {
        objective: "Analyze sales data and create a report".to_string(),
        status: TaskStatus::NotStarted,
        steps_completed: 0,
        steps_total: 5,
        current_step: Some("Load and explore data".to_string()),
    })
    .build();

// Load data
env.update_from_tool_result("read_csv", r#"
{
    "path": "/Users/josh/data_analysis/sales_data.csv",
    "rows": 1250,
    "columns": ["date", "product", "region", "sales", "profit"],
    "sample": [
        {"date": "2025-01-15", "product": "WidgetA", "region": "North", "sales": 1200, "profit": 360},
        {"date": "2025-01-16", "product": "WidgetB", "region": "South", "sales": 950, "profit": 285}
    ]
}
"#).unwrap();

// Create a typed dataset extension
let sales_dataset = Dataset {
    name: "sales_data".to_string(),
    rows: 1250,
    columns: vec!["date".to_string(), "product".to_string(), "region".to_string(), 
                 "sales".to_string(), "profit".to_string()],
    summary_stats: HashMap::new(),
};

env.set_extension("datasets", HashMap::from([
    ("sales_data".to_string(), sales_dataset)
])).unwrap();

// Analyze data
env.update_from_tool_result("analyze_data", r#"
{
    "dataset": "sales_data.csv",
    "summary_stats": {
        "total_sales": 1250000,
        "avg_profit_margin": 0.28,
        "sales_by_region": {
            "North": 450000,
            "South": 380000,
            "East": 320000,
            "West": 100000
        }
    }
}
"#).unwrap();

// Access type-safe environment data
let task_status = env.task().status.clone();
let datasets: HashMap<String, Dataset> = env.get_extension("datasets").unwrap();
let sales_dataset = &datasets["sales_data"];
println!("Analyzing {} rows of sales data", sales_dataset.rows);
```

## Benefits of the Strongly Typed Approach

1. **Type Safety**: Compile-time checks for common operations
2. **IDE Support**: Better autocomplete and documentation
3. **Clear Domain Model**: Makes the environment structure explicit
4. **Error Prevention**: Reduces runtime errors from type mismatches
5. **Performance**: Can be faster than dynamic JSON operations
6. **Flexibility**: Still supports dynamic extensions where needed

The hybrid approach with strongly typed core sections and flexible extensions gives us the best of both worlds - type safety for common operations while maintaining the ability to handle arbitrary data when needed.
