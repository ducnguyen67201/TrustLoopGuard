import type { HealthcareDemoLocale } from './healthcare/content';

interface ContextualPageCopy {
  homeLabel: string;
  personalizedConcept: string;
  liveDemo: string;
  preparedFor: (companyName: string) => string;
  heading: string;
  publicSource: string;
  title: (companyName: string) => string;
  fallbackTitle: string;
  description: (companyName: string) => string;
  fallbackDescription: string;
  disclaimer: (companyName: string) => string;
}

interface ContextualUiCopy {
  greeting: (workflow: string) => string;
  inventoryRequestFailed: string;
  workflowFailed: string;
  dailyLimit: string;
  syntheticBanner: string;
  contextualAgent: string;
  visitor: string;
  replyStopped: string;
  examplesLabel: string;
  messageLabel: string;
  runningWorkflow: string;
  send: string;
  monitorKicker: string;
  monitorTitle: string;
  protectedConversation: string;
  inputBoundary: string;
  inputDetail: string;
  modelDetail: string;
  modelSkipped: string;
  outputBoundary: string;
  outputDetail: string;
  policiesChecked: string;
  loadingRegistry: string;
  inventoryUnavailable: string;
  checkingNow: string;
  matched: string;
  ready: string;
  decisionRecord: string;
  checking: string;
  unavailable: string;
  skipped: string;
  called: string;
  humanReview: string;
  trace: string;
  failedClosed: string;
  waitingForMessage: string;
  inputAndOutputChecked: string;
  stoppedBeforeModel: string;
  loading: string;
  policiesFromRust: (count: number) => string;
  progressInput: string;
  progressModel: string;
  progressOutput: string;
  noViolation: string;
  policyBlocked: string;
  policyTransformed: string;
  policyNeedsReview: string;
  policyDeferred: string;
  matchedPolicy: (policyId: string) => string;
  effect: Record<'permit' | 'transform' | 'deny' | 'require_approval' | 'defer', string>;
  phase: Record<'input' | 'output', string>;
  severity: Record<'low' | 'medium' | 'high' | 'critical', string>;
  action: Record<string, string>;
  policyDescriptions: Record<string, string>;
}

export const CONTEXTUAL_PAGE_COPY: Record<HealthcareDemoLocale, ContextualPageCopy> = {
  en: {
    homeLabel: 'Featherlane AI home',
    personalizedConcept: 'Personalized concept',
    liveDemo: 'Live product demo',
    preparedFor: (companyName) => `Prepared for ${companyName}`,
    heading: 'Your workflow. A policy boundary before the action.',
    publicSource: 'Public-source concept',
    title: (companyName) => `${companyName} AI Guardrail Concept`,
    fallbackTitle: 'Personalized AI Guardrail Demo',
    description: (companyName) =>
      `A private, public-source Featherlane AI concept for ${companyName}.`,
    fallbackDescription: 'A private Featherlane AI concept demo.',
    disclaimer: (companyName) =>
      `This is a concept based on public material and is not connected to ${companyName} or its systems.`,
  },
  vi: {
    homeLabel: 'Trang chủ Featherlane AI',
    personalizedConcept: 'Bản thử nghiệm riêng',
    liveDemo: 'Xem bản thử nghiệm sản phẩm',
    preparedFor: (companyName) => `Dành riêng cho ${companyName}`,
    heading: 'Quy trình của đơn vị. Chính sách kiểm soát trước khi hành động.',
    publicSource: 'Bản thử nghiệm từ nguồn công khai',
    title: (companyName) => `Bản thử nghiệm AI an toàn cho ${companyName}`,
    fallbackTitle: 'Bản thử nghiệm AI an toàn được cá nhân hóa',
    description: (companyName) =>
      `Bản thử nghiệm Featherlane AI riêng cho ${companyName}, được xây dựng từ nguồn công khai.`,
    fallbackDescription: 'Bản thử nghiệm Featherlane AI được cá nhân hóa.',
    disclaimer: (companyName) =>
      `Đây là bản thử nghiệm dựa trên nguồn công khai, không liên kết với ${companyName} hoặc hệ thống của đơn vị này.`,
  },
};

export const CONTEXTUAL_UI_COPY: Record<HealthcareDemoLocale, ContextualUiCopy> = {
  en: {
    greeting: (workflow) =>
      `I’m a synthetic assistant for ${workflow}. Ask a read-only question, request a shared change, or test the authorization boundary.`,
    inventoryRequestFailed: 'Policy inventory request failed',
    workflowFailed: 'The protected contextual workflow failed safely.',
    dailyLimit: 'Daily contextual demo limit reached. Try again tomorrow.',
    syntheticBanner:
      'Synthetic concept only. Do not enter credentials, secrets, or private company data.',
    contextualAgent: 'Contextual agent',
    visitor: 'Visitor',
    replyStopped: 'Reply stopped safely',
    examplesLabel: 'Example contextual requests',
    messageLabel: 'Message',
    runningWorkflow: 'Running protected workflow',
    send: 'Send through Featherlane AI',
    monitorKicker: 'Shared demo workspace',
    monitorTitle: 'Featherlane AI policy monitor',
    protectedConversation: 'Protected conversation',
    inputBoundary: 'Input boundary',
    inputDetail: 'Checks the visitor message before OpenAI is called.',
    modelDetail: 'Uses bounded server-side workflow context and untrusted chat history.',
    modelSkipped: 'Skipped because the input decision stopped the request.',
    outputBoundary: 'Output boundary',
    outputDetail: 'Checks the drafted response before it reaches the visitor.',
    policiesChecked: 'Policies checked',
    loadingRegistry: 'Loading the Rust policy registry…',
    inventoryUnavailable: 'Policy inventory unavailable. Chat checks still fail closed.',
    checkingNow: 'Checking now',
    matched: 'Matched',
    ready: 'Ready',
    decisionRecord: 'Decision record',
    checking: 'Checking',
    unavailable: 'Unavailable',
    skipped: 'Skipped',
    called: 'Called',
    humanReview: 'Human review',
    trace: 'Trace',
    failedClosed: 'Failed closed',
    waitingForMessage: 'Waiting for a message',
    inputAndOutputChecked: 'Input and output checked',
    stoppedBeforeModel: 'Stopped before model',
    loading: 'Loading',
    policiesFromRust: (count) => `${count} from Rust`,
    progressInput: 'Featherlane AI is checking the visitor message…',
    progressModel: 'OpenAI is drafting with bounded workflow context…',
    progressOutput: 'Featherlane AI is checking the drafted reply…',
    noViolation: 'No policy violation was found.',
    policyBlocked: 'A policy blocked this turn.',
    policyTransformed: 'The reply was revised by policy.',
    policyNeedsReview: 'A policy requires human review.',
    policyDeferred: 'This turn was deferred by policy.',
    matchedPolicy: (policyId) => `Policy \`${policyId}\` matched.`,
    effect: {
      permit: 'Permit',
      transform: 'Transform',
      deny: 'Deny',
      require_approval: 'Human review',
      defer: 'Human review',
    },
    phase: { input: 'input', output: 'output' },
    severity: { low: 'low', medium: 'medium', high: 'high', critical: 'critical' },
    action: {
      permit: 'permit',
      deny: 'deny',
      defer: 'defer',
      transform: 'transform',
      check: 'check',
    },
    policyDescriptions: {},
  },
  vi: {
    greeting: (workflow) =>
      `Tôi là trợ lý giả lập cho quy trình: ${workflow} Bạn có thể yêu cầu tra cứu, đề xuất thay đổi cần duyệt hoặc thử giới hạn phân quyền.`,
    inventoryRequestFailed: 'Không thể tải danh sách chính sách',
    workflowFailed: 'Quy trình được bảo vệ đã dừng an toàn.',
    dailyLimit: 'Đã đạt giới hạn dùng thử hôm nay. Vui lòng thử lại vào ngày mai.',
    syntheticBanner:
      'Chỉ dùng cho bản thử nghiệm giả lập. Không nhập thông tin đăng nhập, bí mật hoặc dữ liệu nội bộ thật.',
    contextualAgent: 'Trợ lý giả lập',
    visitor: 'Khách',
    replyStopped: 'Phản hồi đã được dừng an toàn',
    examplesLabel: 'Các yêu cầu giả lập',
    messageLabel: 'Tin nhắn',
    runningWorkflow: 'Đang chạy quy trình được bảo vệ',
    send: 'Gửi qua Featherlane AI',
    monitorKicker: 'Không gian thử nghiệm dùng chung',
    monitorTitle: 'Trình giám sát chính sách Featherlane AI',
    protectedConversation: 'Cuộc trò chuyện được bảo vệ',
    inputBoundary: 'Biên đầu vào',
    inputDetail: 'Kiểm tra tin nhắn của khách trước khi gọi OpenAI.',
    modelDetail: 'Chỉ dùng ngữ cảnh quy trình giới hạn từ máy chủ và coi lịch sử trò chuyện là dữ liệu chưa đáng tin cậy.',
    modelSkipped: 'Đã bỏ qua vì quyết định đầu vào dừng yêu cầu trước khi gọi mô hình.',
    outputBoundary: 'Biên đầu ra',
    outputDetail: 'Kiểm tra bản nháp trước khi gửi tới khách.',
    policiesChecked: 'Các chính sách được kiểm tra',
    loadingRegistry: 'Đang tải danh sách chính sách từ Rust…',
    inventoryUnavailable:
      'Không thể tải danh sách chính sách. Kiểm tra trò chuyện vẫn đóng an toàn khi có lỗi.',
    checkingNow: 'Đang kiểm tra',
    matched: 'Đã khớp',
    ready: 'Sẵn sàng',
    decisionRecord: 'Hồ sơ quyết định',
    checking: 'Đang kiểm tra',
    unavailable: 'Không khả dụng',
    skipped: 'Đã bỏ qua',
    called: 'Đã gọi',
    humanReview: 'Chờ người duyệt',
    trace: 'Truy vết',
    failedClosed: 'Đã đóng an toàn',
    waitingForMessage: 'Đang chờ tin nhắn',
    inputAndOutputChecked: 'Đã kiểm tra đầu vào và đầu ra',
    stoppedBeforeModel: 'Đã dừng trước khi gọi mô hình',
    loading: 'Đang tải',
    policiesFromRust: (count) => `${count} chính sách từ Rust`,
    progressInput: 'Featherlane AI đang kiểm tra tin nhắn của khách…',
    progressModel: 'OpenAI đang soạn phản hồi với ngữ cảnh quy trình giới hạn…',
    progressOutput: 'Featherlane AI đang kiểm tra bản nháp trước khi gửi…',
    noViolation: 'Không phát hiện vi phạm chính sách.',
    policyBlocked: 'Một chính sách đã chặn lượt này.',
    policyTransformed: 'Phản hồi đã được điều chỉnh theo chính sách.',
    policyNeedsReview: 'Một chính sách yêu cầu con người xem xét.',
    policyDeferred: 'Lượt này đã được tạm hoãn để chờ duyệt.',
    matchedPolicy: (policyId) => `Đã khớp chính sách \`${policyId}\`.`,
    effect: {
      permit: 'Cho phép',
      transform: 'Điều chỉnh',
      deny: 'Từ chối',
      require_approval: 'Chờ người duyệt',
      defer: 'Chờ người duyệt',
    },
    phase: { input: 'đầu vào', output: 'đầu ra' },
    severity: { low: 'thấp', medium: 'trung bình', high: 'cao', critical: 'nghiêm trọng' },
    action: {
      permit: 'cho phép',
      deny: 'từ chối',
      defer: 'tạm hoãn',
      transform: 'điều chỉnh',
      check: 'kiểm tra',
    },
    policyDescriptions: {
      'contextual-readonly-input':
        'Nhận diện yêu cầu chỉ đọc nằm trong phạm vi quy trình giả lập.',
      'contextual-shared-change-input':
        'Tạm dừng yêu cầu thay đổi trạng thái dùng chung để con người phê duyệt.',
      'contextual-control-bypass-input':
        'Chặn yêu cầu bỏ qua kiểm soát hoặc dùng thông tin đăng nhập của con người.',
      'contextual-secret-output':
        'Chặn bản nháp làm lộ thông tin đăng nhập, mã truy cập hoặc hồ sơ riêng tư.',
      'contextual-false-execution-output':
        'Thay tuyên bố không có căn cứ rằng bản thử nghiệm đã truy cập hoặc thay đổi hệ thống thật.',
    },
  },
};
