import type {
  AgentToolResult,
  ExtensionAPI,
  Theme,
} from "@earendil-works/pi-coding-agent";
import {
  Editor,
  type EditorTheme,
  Key,
  matchesKey,
  type Component,
  type Focusable,
  Text,
  truncateToWidth,
  type TUI,
  visibleWidth,
  wrapTextWithAnsi,
} from "@earendil-works/pi-tui";
import {
  compactQuestionDisplay,
  sanitizeQuestionDisplay,
} from "./display.js";
import {
  AskUserQuestionsParameters,
  type AskUserQuestionsParams,
  type UserQuestion,
  type UserQuestionOption,
} from "./schema.js";

const OTHER_OPTION_LABEL = "Other (write your own answer)";

type AnswerSource = "option" | "custom";

export interface UserQuestionAnswer {
  id: string;
  value: string;
  source: AnswerSource;
  label?: string;
  /** Extra comment the user attached to this question's answer. */
  note?: string;
}

/** Details returned by the user questions tool and used by its renderer. */
export interface AskUserQuestionsDetails {
  questions: AskUserQuestionsParams["questions"];
  answers: UserQuestionAnswer[];
  cancelled: boolean;
}

interface UserQuestionInteractionResult {
  answers: UserQuestionAnswer[];
  cancelled: boolean;
}

interface RenderOption {
  label: string;
  option?: UserQuestionOption;
  isOther: boolean;
}

function toolResult(
  text: string,
  details: AskUserQuestionsDetails,
): AgentToolResult<AskUserQuestionsDetails> {
  return {
    content: [{ type: "text", text }],
    details,
  };
}

function answerText(answer: UserQuestionAnswer): string {
  const value =
    answer.source === "option" && answer.label !== undefined
      ? `${answer.label} (${answer.value})`
      : answer.value;
  return `${answer.id}: ${sanitizeQuestionDisplay(value)}`;
}

function answersText(answers: readonly UserQuestionAnswer[]): string {
  return answers
    .flatMap((answer) => {
      const lines = [answerText(answer)];
      if (answer.note !== undefined) {
        lines.push(
          `${answer.id} note: ${sanitizeQuestionDisplay(answer.note)}`,
        );
      }
      return lines;
    })
    .join("\n");
}

function cancelledResult(
  questions: AskUserQuestionsParams["questions"],
  answers: UserQuestionAnswer[],
): AgentToolResult<AskUserQuestionsDetails> {
  const answerSummary = answersText(answers);
  const text =
    answerSummary.length === 0
      ? "User cancelled the questions"
      : `User cancelled the questions after answering:\n${answerSummary}`;
  return toolResult(text, { questions, answers, cancelled: true });
}

function questionTitle(question: UserQuestion, index: number, total: number): string {
  return `${index + 1}/${total} [${sanitizeQuestionDisplay(question.id)}] ${sanitizeQuestionDisplay(question.question)}`;
}

function optionLabel(option: UserQuestionOption, index: number): string {
  const description =
    option.description === undefined
      ? ""
      : ` — ${sanitizeQuestionDisplay(option.description)}`;
  return `${index + 1}. ${sanitizeQuestionDisplay(option.label)}${description}`;
}

function isOtherOptionLabel(label: string): boolean {
  const normalized = sanitizeQuestionDisplay(label).trim().toLowerCase();
  return normalized === "other" || normalized === "其他";
}

class UserQuestionsComponent implements Component, Focusable {
  private currentIndex = 0;
  private optionIndex = 0;
  private inputMode = false;
  private cachedWidth?: number;
  private cachedLines?: string[];
  private completed = false;
  private _focused = false;
  private readonly answers = new Map<string, UserQuestionAnswer>();
  private readonly notes = new Map<string, string>();
  private noteMode = false;
  private readonly editor: Editor;

  public constructor(
    private readonly tui: TUI,
    private readonly theme: Theme,
    private readonly questions: AskUserQuestionsParams["questions"],
    private readonly done: (result: UserQuestionInteractionResult | null) => void,
  ) {
    const editorTheme: EditorTheme = {
      borderColor: (value) => theme.fg("accent", value),
      selectList: {
        selectedPrefix: (value) => theme.fg("accent", value),
        selectedText: (value) => theme.fg("accent", value),
        description: (value) => theme.fg("muted", value),
        scrollInfo: (value) => theme.fg("dim", value),
        noMatch: (value) => theme.fg("warning", value),
      },
    };
    this.editor = new Editor(tui, editorTheme);
    this.editor.onSubmit = (value) => {
      if (this.noteMode) {
        this.saveNote(value);
        return;
      }
      this.saveInputAndNavigate(1, value);
    };
    this.activateCurrentQuestion();
  }

  public get focused(): boolean {
    return this._focused;
  }

  public set focused(value: boolean) {
    this._focused = value;
    this.editor.focused = value && (this.inputMode || this.noteMode);
  }

  public cancel(): void {
    this.finish(true);
  }

  public render(width: number): string[] {
    if (this.cachedLines !== undefined && this.cachedWidth === width) {
      return this.cachedLines;
    }

    const renderWidth = Math.max(1, width);
    const lines: string[] = [];
    const currentQuestion = this.currentQuestion();
    const options =
      currentQuestion === undefined ? [] : this.renderOptions(currentQuestion);

    const addWrapped = (text: string): void => {
      lines.push(...wrapTextWithAnsi(text, renderWidth));
    };
    const addWrappedWithPrefix = (prefix: string, text: string): void => {
      const prefixWidth = visibleWidth(prefix);
      if (prefixWidth >= renderWidth) {
        addWrapped(prefix + text);
        return;
      }
      const wrapped = wrapTextWithAnsi(text, renderWidth - prefixWidth);
      const continuationPrefix = " ".repeat(prefixWidth);
      for (let index = 0; index < wrapped.length; index += 1) {
        lines.push(`${index === 0 ? prefix : continuationPrefix}${wrapped[index]}`);
      }
    };

    lines.push(this.theme.fg("accent", "─".repeat(renderWidth)));
    this.renderNavigation(lines, addWrappedWithPrefix);

    if (currentQuestion === undefined) {
      this.renderSubmit(lines, addWrappedWithPrefix);
    } else {
      this.renderQuestion(
        lines,
        renderWidth,
        currentQuestion,
        options,
        addWrappedWithPrefix,
      );
    }

    lines.push("");
    addWrappedWithPrefix(" ", this.theme.fg("dim", this.helpText(currentQuestion)));
    lines.push(this.theme.fg("accent", "─".repeat(renderWidth)));

    this.cachedWidth = width;
    this.cachedLines = lines.map((line) => truncateToWidth(line, renderWidth, ""));
    return this.cachedLines;
  }

  public handleInput(data: string): void {
    if (matchesKey(data, Key.ctrl("c"))) {
      this.finish(true);
      return;
    }

    if (this.noteMode) {
      if (matchesKey(data, Key.escape)) {
        this.exitNoteMode();
        return;
      }
      this.editor.handleInput(data);
      this.refresh();
      return;
    }

    if (this.inputMode) {
      if (matchesKey(data, Key.up)) {
        const question = this.currentQuestion();
        if (question !== undefined && (question.options ?? []).length > 0) {
          this.discardInput();
          this.optionIndex = Math.max(0, this.optionIndex - 1);
          this.refresh();
          return;
        }
      }
      if (matchesKey(data, Key.escape)) {
        const question = this.currentQuestion();
        this.discardInput();
        if (question !== undefined && (question.options ?? []).length === 0) {
          if (this.currentIndex === 0) this.finish(true);
          else this.navigate(-1);
        } else {
          this.refresh();
        }
        return;
      }
      if (
        matchesKey(data, Key.right) ||
        matchesKey(data, Key.tab)
      ) {
        this.saveInputAndNavigate(1);
        return;
      }
      if (
        matchesKey(data, Key.left) ||
        matchesKey(data, Key.shift("tab"))
      ) {
        this.saveInputAndNavigate(-1);
        return;
      }
      this.editor.handleInput(data);
      this.refresh();
      return;
    }

    if (matchesKey(data, Key.escape)) {
      this.finish(true);
      return;
    }
    if (matchesKey(data, Key.right) || matchesKey(data, Key.tab)) {
      this.navigate(1);
      return;
    }
    if (matchesKey(data, Key.left) || matchesKey(data, Key.shift("tab"))) {
      this.navigate(-1);
      return;
    }

    if (this.currentIndex === this.questions.length) {
      if (matchesKey(data, Key.enter) && this.allAnswered()) {
        this.finish(false);
      }
      return;
    }

    if (matchesKey(data, "n")) {
      this.openNoteMode();
      return;
    }

    const question = this.currentQuestion();
    if (question === undefined) throw new Error("Question navigation is out of range.");

    const options = this.renderOptions(question);
    if (options.length === 0) {
      if (matchesKey(data, Key.enter)) this.openInput();
      return;
    }
    if (matchesKey(data, Key.up)) {
      this.optionIndex = Math.max(0, this.optionIndex - 1);
      const option = options[this.optionIndex];
      if (option?.isOther) this.openInput();
      else this.refresh();
      return;
    }
    if (matchesKey(data, Key.down)) {
      this.optionIndex = Math.min(options.length - 1, this.optionIndex + 1);
      const option = options[this.optionIndex];
      if (option?.isOther) this.openInput();
      else this.refresh();
      return;
    }
    if (matchesKey(data, Key.enter)) {
      const option = options[this.optionIndex];
      if (option === undefined) throw new Error("Question option is out of range.");
      if (option.isOther) this.openInput();
      else {
        if (option.option === undefined) throw new Error("Selected option has no value.");
        this.answers.set(question.id, {
          id: question.id,
          value: option.option.value,
          label: option.option.label,
          source: "option",
        });
        this.navigate(1);
      }
    }
  }

  public invalidate(): void {
    this.cachedWidth = undefined;
    this.cachedLines = undefined;
    this.editor.invalidate();
  }

  private currentQuestion(): UserQuestion | undefined {
    return this.questions[this.currentIndex];
  }

  private renderOptions(question: UserQuestion): RenderOption[] {
    const options = question.options ?? [];
    if (options.length === 0) return [];
    const allowOther = question.allowOther !== false;
    const rendered: RenderOption[] = options.map((option, index) => ({
      label: optionLabel(option, index),
      option,
      isOther: allowOther && isOtherOptionLabel(option.label),
    }));
    if (allowOther && !rendered.some((option) => option.isOther)) {
      rendered.push({ label: OTHER_OPTION_LABEL, isOther: true });
    }
    return rendered;
  }

  private allAnswered(): boolean {
    return this.questions.every((question) => this.answers.has(question.id));
  }

  private orderedAnswers(includeNotes: boolean): UserQuestionAnswer[] {
    return this.questions.flatMap((question) => {
      const answer = this.answers.get(question.id);
      if (answer === undefined) return [];
      const note = this.notes.get(question.id);
      if (note === undefined || !includeNotes) return [answer];
      return [{ ...answer, note }];
    });
  }

  private answerIndex(question: UserQuestion, options: readonly RenderOption[]): number {
    const answer = this.answers.get(question.id);
    if (answer === undefined) return 0;
    if (answer.source === "custom") {
      const otherIndex = options.findIndex((option) => option.isOther);
      return otherIndex === -1 ? 0 : otherIndex;
    }
    const optionIndex = options.findIndex(
      (option) => option.option?.value === answer.value,
    );
    return optionIndex === -1 ? 0 : optionIndex;
  }

  private syncSelection(): void {
    const question = this.currentQuestion();
    if (question === undefined) {
      this.optionIndex = 0;
      return;
    }
    const options = this.renderOptions(question);
    this.optionIndex = Math.min(
      this.answerIndex(question, options),
      Math.max(0, options.length - 1),
    );
  }

  private navigate(delta: number): void {
    const tabCount = this.questions.length + 1;
    this.currentIndex = (this.currentIndex + delta + tabCount) % tabCount;
    this.activateCurrentQuestion();
    this.refresh();
  }

  private activateCurrentQuestion(): void {
    const question = this.currentQuestion();
    if (question === undefined || (question.options ?? []).length > 0) {
      this.inputMode = false;
      this.editor.focused = false;
      this.editor.setText("");
      this.syncSelection();
      return;
    }
    const answer = this.answers.get(question.id);
    this.inputMode = true;
    this.editor.setText(answer?.source === "custom" ? answer.value : "");
    this.editor.focused = this._focused;
  }

  private openInput(): void {
    const question = this.currentQuestion();
    if (question === undefined) throw new Error("Cannot edit the submit view.");
    const answer = this.answers.get(question.id);
    this.inputMode = true;
    this.editor.setText(answer?.source === "custom" ? answer.value : "");
    this.editor.focused = this._focused;
    this.refresh();
  }

  private discardInput(): void {
    this.inputMode = false;
    this.editor.focused = false;
    this.editor.setText("");
    this.syncSelection();
  }

  private cancelInput(): void {
    this.discardInput();
    this.refresh();
  }

  private saveInputAndNavigate(
    delta: number,
    value = this.editor.getText(),
  ): void {
    const question = this.currentQuestion();
    if (question === undefined) throw new Error("Cannot save input outside a question.");
    this.answers.set(question.id, {
      id: question.id,
      value: value.trim(),
      source: "custom",
    });
    this.navigate(delta);
  }

  private finish(cancelled: boolean): void {
    if (this.completed) return;
    this.completed = true;
    this.done({ answers: this.orderedAnswers(!cancelled), cancelled });
  }

  private openNoteMode(): void {
    const question = this.currentQuestion();
    if (question === undefined) throw new Error("Cannot attach a note on the submit view.");
    this.noteMode = true;
    this.editor.setText(this.notes.get(question.id) ?? "");
    this.editor.focused = this._focused;
    this.refresh();
  }

  private saveNote(value: string): void {
    const question = this.currentQuestion();
    if (question === undefined) throw new Error("Cannot save a note on the submit view.");
    const trimmed = value.trim();
    if (trimmed.length === 0) this.notes.delete(question.id);
    else this.notes.set(question.id, trimmed);
    this.exitNoteMode();
  }

  private exitNoteMode(): void {
    this.noteMode = false;
    this.editor.setText("");
    this.editor.focused = false;
    this.refresh();
  }

  private refresh(): void {
    this.invalidate();
    this.tui.requestRender();
  }

  private renderNavigation(
    lines: string[],
    addWrappedWithPrefix: (prefix: string, text: string) => void,
  ): void {
    const tabs = this.questions.map((question, index) => {
      const active = index === this.currentIndex;
      const answered = this.answers.has(question.id);
      const text = ` ${answered ? "■" : "□"} Q${index + 1} `;
      const styled = active
        ? this.theme.bg("selectedBg", this.theme.fg("text", text))
        : this.theme.fg(answered ? "success" : "muted", text);
      return styled;
    });
    const submitText = " ✓ Submit ";
    const submitActive = this.currentIndex === this.questions.length;
    tabs.push(
      submitActive
        ? this.theme.bg("selectedBg", this.theme.fg("text", submitText))
        : this.theme.fg(this.allAnswered() ? "success" : "dim", submitText),
    );
    addWrappedWithPrefix(" ", tabs.join(" "));
    lines.push("");
  }

  private renderQuestion(
    lines: string[],
    renderWidth: number,
    question: UserQuestion,
    options: readonly RenderOption[],
    addWrappedWithPrefix: (prefix: string, text: string) => void,
  ): void {
    addWrappedWithPrefix(
      " ",
      this.theme.fg(
        "muted",
        `Question ${this.currentIndex + 1}/${this.questions.length}`,
      ),
    );
    addWrappedWithPrefix(
      " ",
      this.theme.fg("text", sanitizeQuestionDisplay(question.question)),
    );
    lines.push("");

    if (this.inputMode) {
      for (const option of options) {
        const selected = options.indexOf(option) === this.optionIndex;
        const prefix = selected ? this.theme.fg("accent", "> ") : "  ";
        addWrappedWithPrefix(prefix, this.theme.fg(selected ? "accent" : "text", option.label));
        if (option.option?.description !== undefined) {
          addWrappedWithPrefix(
            "     ",
            this.theme.fg(
              "muted",
              sanitizeQuestionDisplay(option.option.description),
            ),
          );
        }
      }
      if (options.length > 0) lines.push("");
      addWrappedWithPrefix(" ", this.theme.fg("muted", "Your answer:"));
      if (
        question.placeholder !== undefined &&
        this.editor.getText().length === 0
      ) {
        addWrappedWithPrefix(
          " ",
          this.theme.fg("dim", sanitizeQuestionDisplay(question.placeholder)),
        );
      }
      for (const line of this.editor.render(Math.max(1, renderWidth - 2))) {
        lines.push(` ${line}`);
      }
      return;
    }

    if (options.length === 0) {
      const answer = this.answers.get(question.id);
      const value =
        answer === undefined || answer.value.length === 0
          ? "(no answer)"
          : answer.value;
      addWrappedWithPrefix(" ", this.theme.fg("muted", "Answer: "));
      addWrappedWithPrefix(" ", this.theme.fg("text", sanitizeQuestionDisplay(value)));
      this.renderNoteSection(question, renderWidth, lines, addWrappedWithPrefix);
      return;
    }

    const answer = this.answers.get(question.id);
    const savedIndex = this.answerIndex(question, options);
    for (let index = 0; index < options.length; index += 1) {
      const option = options[index];
      if (option === undefined) throw new Error("Question option is out of range.");
      const selected = index === this.optionIndex;
      const saved = index === savedIndex && answer !== undefined;
      const prefix = selected ? this.theme.fg("accent", "> ") : "  ";
      const suffix = saved ? this.theme.fg("success", " ✓") : "";
      let label = option.label;
      if (option.isOther && answer?.source === "custom") {
        label = `${label}: ${sanitizeQuestionDisplay(answer.value)}`;
      }
      addWrappedWithPrefix(
        prefix,
        this.theme.fg(selected ? "accent" : "text", label) + suffix,
      );
      if (option.option?.description !== undefined) {
        addWrappedWithPrefix(
          "     ",
          this.theme.fg("muted", sanitizeQuestionDisplay(option.option.description)),
        );
      }
    }
    this.renderNoteSection(question, renderWidth, lines, addWrappedWithPrefix);
  }

  private renderNoteSection(
    question: UserQuestion,
    renderWidth: number,
    lines: string[],
    addWrappedWithPrefix: (prefix: string, text: string) => void,
  ): void {
    if (this.noteMode) {
      lines.push("");
      addWrappedWithPrefix(" ", this.theme.fg("muted", "Note to model:"));
      for (const line of this.editor.render(Math.max(1, renderWidth - 2))) {
        lines.push(` ${line}`);
      }
      return;
    }
    const note = this.notes.get(question.id);
    if (note !== undefined) {
      lines.push("");
      addWrappedWithPrefix(
        " ",
        this.theme.fg("dim", `Note: ${sanitizeQuestionDisplay(note)}`),
      );
    }
  }

  private renderSubmit(
    lines: string[],
    addWrappedWithPrefix: (prefix: string, text: string) => void,
  ): void {
    addWrappedWithPrefix(" ", this.theme.fg("accent", this.theme.bold("Ready to submit")));
    lines.push("");
    for (const question of this.questions) {
      const answer = this.answers.get(question.id);
      const value =
        answer === undefined || answer.value.length === 0
          ? this.theme.fg("dim", "(unanswered)")
          : this.theme.fg("text", sanitizeQuestionDisplay(answer.value));
      addWrappedWithPrefix(
        " ",
        `${this.theme.fg("muted", `${sanitizeQuestionDisplay(question.id)}: `)}${value}`,
      );
      const note = this.notes.get(question.id);
      if (note !== undefined) {
        addWrappedWithPrefix(
          " ",
          this.theme.fg("dim", `note: ${sanitizeQuestionDisplay(note)}`),
        );
      }
    }
    lines.push("");
    addWrappedWithPrefix(
      " ",
      this.theme.fg(
        this.allAnswered() ? "success" : "warning",
        this.allAnswered() ? "Press Enter to submit" : "Answer every question before submitting",
      ),
    );
  }

  private helpText(question: UserQuestion | undefined): string {
    if (this.noteMode) {
      return "Type your note • Enter save • Esc discard";
    }
    if (this.inputMode) {
      return "←→ or Tab/Shift+Tab save and navigate • Enter save and next • Alt+←/→ move cursor • Esc back/cancel";
    }
    if (question === undefined) {
      return "Enter submit • ←→ or Tab/Shift+Tab navigate • Esc cancel";
    }
    if ((question.options ?? []).length === 0) {
      return "Type your answer • Enter save and next • Esc back/cancel";
    }
    return "↑↓ choose • Enter save/edit • n note • ←→ or Tab/Shift+Tab navigate • Esc cancel";
  }
}

/** Register the model-facing tool for questions whose input mode is configured independently. */
export function registerUserQuestionsTool(pi: ExtensionAPI): void {
  pi.registerTool<typeof AskUserQuestionsParameters, AskUserQuestionsDetails>({
    name: "ask_user_questions",
    label: "Ask User Questions",
    description:
      "Ask the user one or more questions. Configure each question independently: provide options for a selection question, omit options for free text, and set allowOther to control custom answers for that question. The user may attach a note to any answer; it is returned as an additional `<id> note:` line.",
    promptSnippet: "Ask the user configured questions and collect structured answers",
    promptGuidelines: [
      "Use ask_user_questions when progress depends on information only the user can provide.",
      "Configure each ask_user_questions question independently instead of combining unrelated choices into one question.",
    ],
    parameters: AskUserQuestionsParameters,
    executionMode: "sequential",

    async execute(_toolCallId, params, signal, _onUpdate, ctx) {
      if (params.questions.length === 0) {
        return toolResult("Error: No questions provided", {
          questions: params.questions,
          answers: [],
          cancelled: true,
        });
      }
      if (ctx.mode !== "tui") {
        return toolResult(
          "Error: User questions require the interactive TUI",
          { questions: params.questions, answers: [], cancelled: true },
        );
      }
      if (signal?.aborted) throw new Error("User questions were cancelled.");

      const interaction = await ctx.ui.custom<UserQuestionInteractionResult | null>(
        (tui, theme, _keybindings, done) => {
          const component = new UserQuestionsComponent(
            tui,
            theme,
            params.questions,
            done,
          );
          signal?.addEventListener("abort", () => component.cancel(), {
            once: true,
          });
          return component;
        },
      );

      if (interaction === null || interaction.cancelled) {
        return cancelledResult(params.questions, interaction?.answers ?? []);
      }
      return toolResult(answersText(interaction.answers), {
        questions: params.questions,
        answers: interaction.answers,
        cancelled: false,
      });
    },

    renderCall(args, theme) {
      const count = args.questions.length;
      const firstQuestion = args.questions[0];
      const summary =
        count === 1 && firstQuestion !== undefined
          ? compactQuestionDisplay(firstQuestion.question)
          : `${count} questions`;
      return new Text(
        theme.fg("toolTitle", theme.bold("ask_user_questions ")) +
          theme.fg("muted", summary),
        0,
        0,
      );
    },

    renderResult(result, _options, theme) {
      const details = result.details;
      if (details === undefined) {
        const first = result.content[0];
        return new Text(
          first?.type === "text" ? sanitizeQuestionDisplay(first.text) : "",
          0,
          0,
        );
      }
      if (details.cancelled) {
        return new Text(theme.fg("warning", "Cancelled"), 0, 0);
      }
      return new Text(
        details.answers
          .map((answer) => {
            const base =
              theme.fg("success", "✓ ") +
              theme.fg("accent", answerText(answer));
            return answer.note === undefined
              ? base
              : `${base}\n  ${theme.fg("muted", `note: ${sanitizeQuestionDisplay(answer.note)}`)}`;
          })
          .join("\n"),
        0,
        0,
      );
    },
  });
}
