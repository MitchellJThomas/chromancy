import type { ExtensionAPI, ExtensionContext } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
  // Track if we've stashed in this session to avoid double-stashing
  let hasStashedThisSession = false;

  // Helper: check if repo is dirty
  async function isDirty(): Promise<boolean> {
    try {
      const result = await pi.exec("git", ["status", "--porcelain"]);
      return result.stdout.trim().length > 0;
    } catch {
      return false;
    }
  }

  // Helper: check if we're in a git repo
  async function inGitRepo(): Promise<boolean> {
    try {
      const result = await pi.exec("git", ["rev-parse", "--git-dir"]);
      return result.code === 0;
    } catch {
      return false;
    }
  }

  // Helper: get current branch
  async function getBranch(): Promise<string | null> {
    try {
      const result = await pi.exec("git", ["branch", "--show-current"]);
      return result.stdout.trim() || null;
    } catch {
      return null;
    }
  }

  // On session start: show git status
  pi.on("session_start", async (_event, ctx: ExtensionContext) => {
    if (!ctx.hasUI) return;

    if (!(await inGitRepo())) {
      ctx.ui.setStatus("git-workflow", "Not a git repo");
      return;
    }

    const branch = await getBranch();
    const dirty = await isDirty();

    if (branch) {
      const status = dirty ? "●" : "✓";
      ctx.ui.setStatus("git-workflow", `git: ${branch} ${status}`);
    }

    // Warn if on main with dirty state
    if (branch === "main" && dirty) {
      ctx.ui.notify("You have uncommitted changes on main. Consider creating a feature branch.", "warning");
    }
  });

  // Before each turn: auto-stash if dirty (only once per session)
  pi.on("turn_start", async (_event, ctx: ExtensionContext) => {
    if (hasStashedThisSession) return;
    if (!(await inGitRepo())) return;
    if (!(await isDirty())) return;

    const branch = await getBranch();
    if (!branch) return;

    // Don't stash on main — warn instead
    if (branch === "main") {
      if (ctx.hasUI) {
        ctx.ui.notify("Working on main with uncommitted changes. Stash or branch before proceeding.", "warning");
      }
      return;
    }

    try {
      await pi.exec("git", ["stash", "push", "-m", "pi-auto-stash"]);
      hasStashedThisSession = true;
      if (ctx.hasUI) {
        ctx.ui.notify(`Auto-stashed changes on ${branch}`, "info");
      }
    } catch (err) {
      if (ctx.hasUI) {
        ctx.ui.notify("Failed to auto-stash changes", "error");
      }
    }
  });

  // On session exit: offer to commit or restore
  pi.on("session_shutdown", async (event, ctx: ExtensionContext) => {
    if (!(await inGitRepo())) return;
    if (event.reason === "reload") return; // Don't prompt on reload

    const branch = await getBranch();
    const dirty = await isDirty();

    // If we stashed earlier, pop the stash
    if (hasStashedThisSession) {
      try {
        await pi.exec("git", ["stash", "pop"]);
        if (ctx.hasUI) {
          ctx.ui.notify("Restored auto-stashed changes", "info");
        }
      } catch {
        // Stash might have been empty or already popped
      }
    }

    // If there are changes and we're not on main, offer to commit
    if (dirty && branch && branch !== "main" && ctx.hasUI) {
      const ok = await ctx.ui.confirm(
        "Commit changes?",
        `You have uncommitted changes on '${branch}'. Commit before exiting?`,
        { timeout: 10000 }
      );

      if (ok) {
        const message = await ctx.ui.input("Commit message:", `wip: ${branch}`);
        if (message) {
          try {
            await pi.exec("git", ["add", "-A"]);
            await pi.exec("git", ["commit", "-m", message]);
            ctx.ui.notify(`Committed: ${message}`, "success");
          } catch {
            ctx.ui.notify("Commit failed", "error");
          }
        }
      }
    }
  });

  // Register /git-status command
  pi.registerCommand("git-status", {
    description: "Show current git branch and status",
    handler: async (_args, ctx: ExtensionContext) => {
      if (!(await inGitRepo())) {
        ctx.ui.notify("Not in a git repository", "error");
        return;
      }

      const result = await pi.exec("git", ["status", "-sb"]);
      ctx.ui.notify(result.stdout.trim() || "Clean working tree", "info");
    },
  });

  // Register /git-commit command
  pi.registerCommand("git-commit", {
    description: "Commit all changes with a message",
    handler: async (args, ctx: ExtensionContext) => {
      if (!(await inGitRepo())) {
        ctx.ui.notify("Not in a git repository", "error");
        return;
      }

      const branch = await getBranch();
      if (branch === "main") {
        const ok = await ctx.ui.confirm(
          "Commit to main?",
          "You are on main branch. Are you sure?"
        );
        if (!ok) return;
      }

      const message = args || (await ctx.ui.input("Commit message:", `wip: ${branch}`));
      if (!message) return;

      try {
        await pi.exec("git", ["add", "-A"]);
        await pi.exec("git", ["commit", "-m", message]);
        ctx.ui.notify(`Committed: ${message}`, "success");
      } catch {
        ctx.ui.notify("Commit failed", "error");
      }
    },
  });

  // Register /git-branch command
  pi.registerCommand("git-branch", {
    description: "Create and switch to a new feature branch",
    handler: async (args, ctx: ExtensionContext) => {
      if (!(await inGitRepo())) {
        ctx.ui.notify("Not in a git repository", "error");
        return;
      }

      let branchName = args;
      if (!branchName) {
        branchName = await ctx.ui.input("Branch name (feature/ prefix added):", "");
        if (!branchName) return;
      }

      // Add feature/ prefix if no prefix given
      if (!branchName.match(/^(feature|fix|refactor|docs|release)\//)) {
        branchName = `feature/${branchName}`;
      }

      // Check for dirty state
      if (await isDirty()) {
        const ok = await ctx.ui.confirm(
          "Uncommitted changes",
          "Stash current changes before branching?"
        );
        if (ok) {
          await pi.exec("git", ["stash", "push", "-m", "pre-branch-stash"]);
        }
      }

      try {
        await pi.exec("git", ["checkout", "-b", branchName]);
        ctx.ui.notify(`Switched to new branch: ${branchName}`, "success");
      } catch {
        ctx.ui.notify("Failed to create branch", "error");
      }
    },
  });

  // Register /git-pr-ready command
  pi.registerCommand("git-pr-ready", {
    description: "Run PR validation checks",
    handler: async (_args, ctx: ExtensionContext) => {
      const scriptPath = `${ctx.cwd}/.pi/skills/github-pr/scripts/pr-check.sh`;
      try {
        const result = await pi.exec("bash", [scriptPath]);
        ctx.ui.notify("PR checks passed!", "success");
      } catch (err: any) {
        ctx.ui.notify("PR checks failed. See output for details.", "error");
        // Output is already shown by the tool execution
      }
    },
  });
}
