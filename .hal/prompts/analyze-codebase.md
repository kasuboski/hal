Analyze the codebase of the project. Your goal is to familarize yourself with the codebase in order to work more effectively.
Read the project best practices from the `.cursor/rules` directory. They are written in markdown and can be used to guide the analysis.

Focus on the following areas:
1. Documentation: Review the documentation of the codebase. Look for documentation in `docs` as well as in the code itself. Module comments are a good place to start.
2. Code organization: Understand the organization of the codebase. Identify the main modules and their responsibilities.
3. Dependencies: Review the dependencies that are already present. Remember to make use of these if required.
4. Problems: Indentify if the codebase compiles. `direnv exec . cargo check` in the project directory can be used to check the codebase.
