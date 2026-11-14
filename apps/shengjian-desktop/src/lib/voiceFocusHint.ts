/**
 * 「声键窗口在前台时按了语音键」的提示判定。
 *
 * 背景：键盘平台层在**前台窗口属于本进程**时会主动跳过输入法切换
 * （`ime::ImeActivation::SkippedSelfForeground`）——切自己的 WebView 会触发
 * TSF 会话切换导致整页重载（`Bugs/2026-09-12`），而且本窗口里没有可输入文字的
 * 文本框，切了也无处落字。结果是：用户按了语音键、遥控器音频也收到了，但屏幕上
 * 什么都不会发生，现场只能表现为「语音键没反应」。
 *
 * 界面侧补一条提示说明原因和下一步；本模块只做判定，便于单测（捕获边缘多）。
 */

export interface VoiceFocusHintInput {
  /** 当前语音会话状态（engine store 的 `voiceState`）。 */
  voiceState: string;
  /** 上一个语音会话状态，用于只在「开始说话」那一刻提示。 */
  previousVoiceState: string;
  /** 首次启动向导是否打开：它的第 3 步就是在本窗口里试麦克风，属于正常场景。 */
  onboardingOpen: boolean;
  /** 本窗口是否持有焦点（前端用 `document.hasFocus()`）。 */
  windowFocused: boolean;
  /** 本次会话已经提示过，避免按住期间重复弹。 */
  alreadyHinted: boolean;
}

/** 是否应该提示「前台是本窗口，语音不会输入到这里」。 */
export function shouldHintVoiceSelfForeground(input: VoiceFocusHintInput): boolean {
  // 只在 idle/其他 → streaming 的跳变上提示，按住期间重复的 streaming 不再触发。
  if (input.voiceState !== "streaming") return false;
  if (input.previousVoiceState === "streaming") return false;
  // 首次启动向导的麦克风测试本来就在本窗口里做。
  if (input.onboardingOpen) return false;
  if (!input.windowFocused) return false;
  return !input.alreadyHinted;
}
