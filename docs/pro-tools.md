# Enabling Tool Usage for the Pro Agent

## Overview

This document outlines the plan to enable the Pro agent to utilize tools within the HAL codebase. Currently, the Pro agent is limited to generating plans and analyzing results, without the ability to interact with the environment through tools. This enhancement will allow the Pro agent to dynamically gather information, test code, and perform other actions, leading to more effective and robust solutions.

We will reuse the existing `AgentExecutor` component to manage the Pro agent's tool interactions, ensuring consistency and minimizing code duplication.

## Goals

*   Enable the Pro agent to use tools for planning and analysis.
*   Reuse the existing `AgentExecutor` component.
*   Maintain the existing functionality of the Junior agent.
*   Ensure clear error handling and event reporting.
*   Extract a coherent plan from tool-based interactions.

## Implementation Plan

### 1. Modify `CoderConfig` (in `src/coder/config.rs`)

*   Add a `pro_tool_defs` field to the `CoderConfig` struct:

    ```rust
    pub struct CoderConfig<C>
    where
        C: CompletionModel + Clone + Send + Sync + 'static,
    {
        // Existing fields...
        pub pro_agent: Agent<C>,
        pub junior_agent: Arc<Agent<C>>,
        pub tool_defs: Arc<Vec<ToolDefinition>>,
        pub max_junior_iterations: usize,

        // New fields
        pub pro_tool_defs: Arc<Vec<ToolDefinition>>,
        pub max_pro_iterations: usize, // Add a limit for Pro agent iterations
    }
    ```

*   Update the `CoderConfig::new` function to accept the new arguments and initialize the fields:

    ```rust
    impl<C> CoderConfig<C>
    where
        C: CompletionModel + Clone + Send + Sync + 'static,
    {
        pub fn new(
            pro_agent: Agent<C>,
            junior_agent: Agent<C>,
            tool_defs: Vec<ToolDefinition>,
            max_junior_iterations: usize,
            pro_tool_defs: Vec<ToolDefinition>, // New argument
            max_pro_iterations: usize, // New argument
        ) -> Self {
            Self {
                pro_agent,
                junior_agent: Arc::new(junior_agent),
                tool_defs: Arc::new(tool_defs),
                max_junior_iterations,
                pro_tool_defs: Arc::new(pro_tool_defs), // Initialize new field
                max_pro_iterations, // Initialize new field
            }
        }
    }
    ```

### 2. Modify `run` function (in `src/coder/session.rs`)

*   **Remove `run_pro_completion`:** This function is no longer needed as we will use the `AgentExecutor`.

*   **Create Pro Agent Executor:** Create a new `AgentExecutor` instance for the Pro agent:

    ```rust
    let pro_agent_arc = Arc::new(config.pro_agent.clone());
    let mut pro_executor = AgentExecutor::new(
        pro_agent_arc,
        config.pro_tool_defs.clone(),
        config.max_pro_iterations,
    );
    ```

*   **Execute Pro Agent for Planning:** Use the `AgentExecutor` to execute the Pro agent's planning step. This will involve:
    *   Creating an appropriate initial prompt for the Pro agent that instructs it to:
        - Use tools to gather necessary information
        - Put the complete, structured plan in the "finish" tool's summary parameter
    *   Set up a channel to receive executor events:
        ```rust
        let (tx, mut rx) = mpsc::channel(32);
        ```
    *   Send the prompt to the `AgentExecutor`:
        ```rust
        let executor_handle = tokio::spawn(async move {
            pro_executor.execute(planning_prompt, tx).await
        });
        ```
    *   Listen for events and handle them appropriately:
        ```rust
        let mut plan: Option<String> = None;
        while let Some(event_result) = rx.recv().await {
            match event_result {
                Ok(ExecutorEvent::Thinking { text }) => {
                    // Convert to CoderEvent::ProThinking
                    event_sender.send(CoderEvent::ProThinking { text }).await?;
                },
                Ok(ExecutorEvent::ToolCallAttempted { call }) => {
                    // Convert to appropriate CoderEvent
                    event_sender.send(CoderEvent::ProToolCall {
                        tool: call.function.name.clone(),
                        args: call.function.arguments.to_string(),
                    }).await?;
                },
                Ok(ExecutorEvent::ToolCallCompleted { id, result, tool_name }) => {
                    // Convert to appropriate CoderEvent
                    event_sender.send(CoderEvent::ProToolResult {
                        tool: tool_name,
                        result: result.clone(),
                    }).await?;
                },
                Ok(ExecutorEvent::Finished { summary }) => {
                    // Extract the plan from the finish tool's summary
                    plan = Some(summary.clone());
                    event_sender.send(CoderEvent::ProPlanGenerated {
                        plan: summary.clone()
                    }).await?;
                },
                Ok(ExecutorEvent::ExecutionError { error }) => {
                    // Handle non-fatal errors
                    event_sender.send(CoderEvent::Warning {
                        message: format!("Pro agent tool error: {}", error)
                    }).await?;
                },
                Err(e) => {
                    // Handle fatal errors
                    return Err(e);
                }
            }
        }

        // Wait for executor to complete
        let outcome = executor_handle.await.map_err(|e| {
            CoderError::JoinError(format!("Failed to join Pro agent executor: {}", e))
        })?;

        // Ensure we have a plan
        let plan = plan.ok_or_else(|| {
            CoderError::AgentError("Pro agent did not generate a plan".to_string())
        })?;
        ```

*   **Execute Pro Agent for Analysis:** Similarly, use the `AgentExecutor` to execute the Pro agent's analysis step after the Junior agent has completed, extracting the analysis from the finish tool's summary.

*   **Error Handling:** Ensure that errors from the Pro agent's `AgentExecutor` are properly caught and handled, and converted to `CoderEvent::SessionFailed` if necessary.

### 3. Update `run_coder_session` (in `src/coder.rs`)

*   Update the function signature of `run_coder_session` to accept the modified `CoderConfig` with the new fields:
    ```rust
    pub async fn run_coder_session<C>(
        task: String,
        config: CoderConfig<C>,
        event_sender: Sender<CoderEvent>,
    ) -> Result<(), CoderError>
    where
        C: CompletionModel + Clone + Send + Sync + 'static,
    ```

### 4. Pro Agent Prompting

* Create a prompt for the Pro agent that clearly instructs it to use the "finish" tool to submit its final plan:

    ```
    Your task is to develop a comprehensive plan to solve the following problem:

    {task}

    You have access to various tools that can help you gather information and make informed decisions.

    Your goal is to:
    1. Use available tools to gather necessary information about the codebase, requirements, and potential approaches
    2. Develop a clear, step-by-step plan that a junior developer can follow to implement the solution
    3. When you have finalized your plan, call the "finish" tool with your complete plan in the summary parameter

    The plan should include:
    - Clear steps for implementation
    - Key technical considerations
    - Potential edge cases to handle

    IMPORTANT: Your plan must be complete and detailed in the summary of the finish tool call. This plan will be given to another agent to implement.
    ```

## Implementation Considerations

*   **Using "finish" Tool for Plan Extraction:** We will use the existing "finish" tool rather than creating a custom plan submission tool. The Pro agent will be prompted to include its complete, structured plan in the summary parameter of the finish tool.

*   **Plan Extraction Mechanics:** When the `ExecutorEvent::Finished` event is received, the summary parameter will be extracted and used as the plan. This requires ensuring that the Pro agent puts a complete, well-structured plan in this parameter.

*   **Max Iterations for Pro Agent:** Set an appropriate value for `max_pro_iterations` to prevent infinite loops. This should be high enough to allow for complex information gathering but not so high that it allows runaway processes.

*   **Event Conversion:** Create new `CoderEvent` types or adapt existing ones to represent Pro agent tool usage.

*   **Prompt Engineering:** The prompt must clearly instruct the Pro agent to use tools for information gathering and to submit its final plan via the finish tool's summary parameter.

*   **Tool Selection:** Determine which tools are appropriate for the Pro agent to use. Consider read-only tools for the initial implementation to reduce risk.

*   **Transition Period:** Consider a graduated approach where some planning tasks use the old method and others use the new tool-enabled approach, based on a feature flag.

## Testing Strategy

*   **Unit Tests:** Test the `AgentExecutor` with the Pro agent using mock tool responses. Consider `src/model/mock_model.rs` for a mock completion model.

*   **Integration Tests:** Test the entire `run_coder_session` flow with the Pro agent using tools.

*   **Plan Extraction Tests:** Specifically test that plans can be properly extracted from the finish tool's summary parameter.

*   **Error Handling Tests:** Verify that errors from the Pro agent's tool usage are properly handled.

*   **Prompt Effectiveness Tests:** Test different prompting strategies to ensure the Pro agent uses tools effectively and generates clear plans.

## Future Enhancements

*   **Custom Plan Submission Tool:** Consider creating a dedicated `submit_plan` tool with structured fields for more reliable plan extraction.

*   **Dynamic Tool Selection:** Allow the Pro agent to dynamically select tools based on the task at hand.

*   **Tool Call Analytics:** Track the usage of tools by the Pro agent to identify patterns and optimize the toolset.

*   **Improved Prompting:** Experiment with different prompting strategies to improve the Pro agent's performance.

*   **Two-Phase Execution:** Explore a model where the Pro agent first gathers information with tools, then consolidates findings into a plan without tools.
