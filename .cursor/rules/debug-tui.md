---
title: Debugging Tui Issues
description: When you need to debug tui problems, you can output logs using an environment variable
---

When encountering issues with the terminal user interface (TUI), you can enable debug logging to help diagnose problems:

1. Set the `HAL_TUI_DEBUG` environment variable before running the application:

   ```bash
   # On Unix/Linux/macOS
   export HAL_TUI_DEBUG=1
   ./hal

   # Or in a single command
   HAL_TUI_DEBUG=1 ./hal

   # On Windows PowerShell
   $env:HAL_TUI_DEBUG=1
   ./hal
   ```

2. Debug logs will be written to `hal-debug.log` in the current directory.

3. The log file contains detailed information about:
   - Cursor positioning and movement
   - Scroll position calculations
   - Mouse event handling
   - Input field state changes

4. To disable debug logging, simply run the application without the environment variable set.

Note: Debug logs are only written when the `HAL_TUI_DEBUG` environment variable is set, so no log file will be created during normal operation. 