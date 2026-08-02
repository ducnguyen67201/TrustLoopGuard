import type { MarketingLocale } from '@/lib/marketing-locale';

export type RefundDecision =
  | 'ready'
  | 'running'
  | 'executed'
  | 'held'
  | 'blocked'
  | 'checked';

interface RefundExampleCopy {
  label: string;
  prompt: string;
}

interface RefundPageCopy {
  title: string;
  description: string;
  homeLabel: string;
  stripeTestMode: string;
  viewSource: string;
  eyebrow: string;
  heading: string;
  introduction: string;
  healthcareDemo: string;
  safetyLabel: string;
  safetyNote: string;
}

interface RefundUiCopy {
  pagePath: '/demo' | '/vi/demo';
  examples: readonly [RefundExampleCopy, RefundExampleCopy, RefundExampleCopy];
  workflowFailed: string;
  dailyLimit: string;
  customerSupport: string;
  chatTitle: string;
  live: string;
  supportAgent: string;
  greetingBeforeOrder: string;
  greetingAfterOrder: string;
  customer: string;
  agentWorking: string;
  approvalRequired: string;
  approvalDescription: string;
  reviewAction: string;
  errorTitle: string;
  examplesLabel: string;
  messageLabel: string;
  runningWorkflow: string;
  runRefund: string;
  executionTrace: string;
  controlBoundary: string;
  openAiAgent: string;
  openAiDetail: string;
  orderEvidence: string;
  orderEvidenceIdle: string;
  orderEvidenceComplete: string;
  guardDetail: string;
  guardComplete: Record<RefundDecision, string>;
  stripeTestMode: string;
  stripeExecuted: string;
  stripeHeld: string;
  stripeBlocked: string;
  stripeDefault: string;
  seededOrder: string;
  captured: string;
  refundable: string;
  payment: string;
  decisionProof: string;
  waitingForAction: string;
  decision: string;
  receipt: string;
  notCreated: string;
  provider: string;
  notCalled: string;
  decisionLabels: Record<RefundDecision, string>;
}

export const REFUND_PAGE_COPY: Record<MarketingLocale, RefundPageCopy> = {
  en: {
    title: 'Live AI Refund Guard Demo',
    description:
      'Ask a live AI support agent to issue a refund and watch Featherlane AI authorize, hold, or block the Stripe action before it executes.',
    homeLabel: 'Featherlane AI home',
    stripeTestMode: 'Stripe test mode',
    viewSource: 'View source',
    eyebrow: 'Live refund demo',
    heading: 'Ask the agent. Watch the guard decide.',
    introduction:
      'Pick an amount below. Featherlane AI allows, holds, or blocks the agent before Stripe — real APIs, not a scripted animation.',
    healthcareDemo: 'Try the healthcare demo',
    safetyLabel: 'Test data — no real money.',
    safetyNote: 'Test data · No real money',
  },
  vi: {
    title: 'Demo kiểm soát hoàn tiền bằng AI trực tiếp',
    description:
      'Yêu cầu tác nhân hỗ trợ AI hoàn tiền và xem Featherlane AI cho phép, giữ để phê duyệt hoặc chặn hành động Stripe trước khi thực thi.',
    homeLabel: 'Trang chủ Featherlane AI',
    stripeTestMode: 'Stripe thử nghiệm',
    viewSource: 'Xem mã nguồn',
    eyebrow: 'Demo hoàn tiền trực tiếp',
    heading: 'Yêu cầu tác nhân. Xem Featherlane AI quyết định.',
    introduction:
      'Chọn một số tiền bên dưới. Featherlane AI cho phép, giữ để phê duyệt hoặc chặn tác nhân trước khi gọi Stripe — API thật, không phải hoạt ảnh dựng sẵn.',
    healthcareDemo: 'Thử demo y tế',
    safetyLabel: 'Dữ liệu thử nghiệm — không dùng tiền thật.',
    safetyNote: 'Dữ liệu thử nghiệm · Không dùng tiền thật',
  },
};

export const REFUND_UI_COPY: Record<MarketingLocale, RefundUiCopy> = {
  en: {
    pagePath: '/demo',
    examples: [
      {
        label: '$25 · auto-allow',
        prompt: 'Refund order ord_demo_1001 for $25 because the item arrived damaged.',
      },
      {
        label: '$75 · hold',
        prompt: 'Refund order ord_demo_1001 for $75 because the item arrived damaged.',
      },
      {
        label: '$125 · block',
        prompt: 'Refund order ord_demo_1001 for $125 because the item arrived damaged.',
      },
    ],
    workflowFailed: 'The live refund workflow failed safely.',
    dailyLimit: 'Daily refund demo limit reached. Try again tomorrow.',
    customerSupport: 'Customer support',
    chatTitle: 'Ask the refund agent',
    live: 'Live',
    supportAgent: 'Support agent',
    greetingBeforeOrder: 'I can look up order',
    greetingAfterOrder:
      'and propose a refund. Every refund must pass Featherlane AI before Stripe can execute it.',
    customer: 'Customer',
    agentWorking: 'OpenAI is choosing and calling the refund tools…',
    approvalRequired: 'Human approval required',
    approvalDescription:
      'Open the exact held action in Featherlane AI. This demo updates automatically.',
    reviewAction: 'Review this exact action',
    errorTitle: 'Refund stopped safely',
    examplesLabel: 'Example refund requests',
    messageLabel: 'Customer message',
    runningWorkflow: 'Running live workflow',
    runRefund: 'Run live refund',
    executionTrace: 'Execution trace',
    controlBoundary: 'The control boundary',
    openAiAgent: 'OpenAI agent',
    openAiDetail: 'Chooses search_order, prepare_refund, and execute_refund tools.',
    orderEvidence: 'Order evidence',
    orderEvidenceIdle: 'Checks the captured order and refundable balance.',
    orderEvidenceComplete: 'Found the order and checked its refundable balance.',
    guardDetail: 'Evaluates amount, grant scope, eligibility evidence, and policy.',
    guardComplete: {
      ready: 'Ready to evaluate the proposed refund.',
      running: 'Evaluating the proposed refund.',
      executed: 'Authorized the refund for execution.',
      held: 'Held the refund for human approval.',
      blocked: 'Blocked the proposed refund.',
      checked: 'Checked the proposed refund.',
    },
    stripeTestMode: 'Stripe test mode',
    stripeExecuted: 'Created the refund after Featherlane AI authorization.',
    stripeHeld: 'Not called. The refund is waiting for human approval.',
    stripeBlocked: 'Not called. Featherlane AI blocked the proposed refund.',
    stripeDefault: 'Creates the refund only after Featherlane AI authorization.',
    seededOrder: 'Seeded order',
    captured: 'Captured',
    refundable: 'Refundable',
    payment: 'Payment',
    decisionProof: 'Decision proof',
    waitingForAction: 'Waiting for a proposed action',
    decision: 'Decision',
    receipt: 'Receipt',
    notCreated: 'Not created',
    provider: 'Provider',
    notCalled: 'Not called',
    decisionLabels: {
      ready: 'Ready',
      running: 'Checking',
      executed: 'Executed',
      held: 'Held',
      blocked: 'Blocked',
      checked: 'Checked',
    },
  },
  vi: {
    pagePath: '/vi/demo',
    examples: [
      {
        label: '$25 · tự động cho phép',
        prompt: 'Hoàn $25 cho đơn hàng ord_demo_1001 vì sản phẩm bị hư hỏng.',
      },
      {
        label: '$75 · chờ phê duyệt',
        prompt: 'Hoàn $75 cho đơn hàng ord_demo_1001 vì sản phẩm bị hư hỏng.',
      },
      {
        label: '$125 · chặn',
        prompt: 'Hoàn $125 cho đơn hàng ord_demo_1001 vì sản phẩm bị hư hỏng.',
      },
    ],
    workflowFailed: 'Quy trình hoàn tiền trực tiếp đã dừng an toàn.',
    dailyLimit: 'Đã đạt giới hạn demo hoàn tiền hôm nay. Vui lòng thử lại vào ngày mai.',
    customerSupport: 'Hỗ trợ khách hàng',
    chatTitle: 'Yêu cầu tác nhân hoàn tiền',
    live: 'Trực tiếp',
    supportAgent: 'Tác nhân hỗ trợ',
    greetingBeforeOrder: 'Tôi có thể tra cứu đơn hàng',
    greetingAfterOrder:
      'và đề xuất hoàn tiền. Mọi khoản hoàn tiền phải qua Featherlane AI trước khi Stripe có thể thực thi.',
    customer: 'Khách hàng',
    agentWorking: 'OpenAI đang chọn và gọi các công cụ hoàn tiền…',
    approvalRequired: 'Cần con người phê duyệt',
    approvalDescription:
      'Mở đúng hành động đang được giữ trong Featherlane AI. Demo này sẽ tự động cập nhật.',
    reviewAction: 'Xem hành động cần phê duyệt',
    errorTitle: 'Đã dừng hoàn tiền an toàn',
    examplesLabel: 'Ví dụ yêu cầu hoàn tiền',
    messageLabel: 'Tin nhắn của khách hàng',
    runningWorkflow: 'Đang chạy quy trình trực tiếp',
    runRefund: 'Chạy hoàn tiền trực tiếp',
    executionTrace: 'Dấu vết thực thi',
    controlBoundary: 'Ranh giới kiểm soát',
    openAiAgent: 'Tác nhân OpenAI',
    openAiDetail: 'Chọn các công cụ search_order, prepare_refund và execute_refund.',
    orderEvidence: 'Bằng chứng đơn hàng',
    orderEvidenceIdle: 'Kiểm tra đơn hàng đã thanh toán và số dư có thể hoàn.',
    orderEvidenceComplete: 'Đã tìm thấy đơn hàng và kiểm tra số dư có thể hoàn.',
    guardDetail: 'Đánh giá số tiền, phạm vi cấp quyền, bằng chứng đủ điều kiện và chính sách.',
    guardComplete: {
      ready: 'Sẵn sàng đánh giá khoản hoàn tiền được đề xuất.',
      running: 'Đang đánh giá khoản hoàn tiền được đề xuất.',
      executed: 'Đã cho phép thực thi khoản hoàn tiền.',
      held: 'Đã giữ khoản hoàn tiền để con người phê duyệt.',
      blocked: 'Đã chặn khoản hoàn tiền được đề xuất.',
      checked: 'Đã kiểm tra khoản hoàn tiền được đề xuất.',
    },
    stripeTestMode: 'Stripe thử nghiệm',
    stripeExecuted: 'Đã tạo khoản hoàn tiền sau khi Featherlane AI cho phép.',
    stripeHeld: 'Chưa gọi Stripe. Khoản hoàn tiền đang chờ con người phê duyệt.',
    stripeBlocked: 'Chưa gọi Stripe. Featherlane AI đã chặn khoản hoàn tiền được đề xuất.',
    stripeDefault: 'Chỉ tạo khoản hoàn tiền sau khi Featherlane AI cho phép.',
    seededOrder: 'Đơn hàng thử nghiệm',
    captured: 'Đã thanh toán',
    refundable: 'Có thể hoàn',
    payment: 'Thanh toán',
    decisionProof: 'Bằng chứng quyết định',
    waitingForAction: 'Đang chờ hành động được đề xuất',
    decision: 'Quyết định',
    receipt: 'Biên nhận',
    notCreated: 'Chưa tạo',
    provider: 'Nhà cung cấp',
    notCalled: 'Chưa gọi',
    decisionLabels: {
      ready: 'Sẵn sàng',
      running: 'Đang kiểm tra',
      executed: 'Đã thực thi',
      held: 'Chờ phê duyệt',
      blocked: 'Đã chặn',
      checked: 'Đã kiểm tra',
    },
  },
};
