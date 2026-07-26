import Link from 'next/link';
import {
  AGENT_SPENDING_CAPS_USE_CASE,
  EMAIL_USE_CASE,
  SHELL_COMMAND_USE_CASE,
} from '@/app/use-cases/content';
import type { MarketingLocale } from '@/lib/marketing-locale';
import { UseCaseShowcase } from './use-case-showcase';

const FEATURED_USE_CASES = [
  AGENT_SPENDING_CAPS_USE_CASE,
  SHELL_COMMAND_USE_CASE,
  EMAIL_USE_CASE,
] as const;

const FEATURED_USE_CASES_VI = [
  {
    ...SHELL_COMMAND_USE_CASE,
    eyebrow: 'Hàng rào an toàn cho lệnh shell',
    title: 'Chặn lệnh shell nguy hiểm trước khi chúng chạy.',
    summary:
      'Đánh giá từng hành động Bash, sh hoặc zsh được đề xuất dưới dạng dữ liệu có cấu trúc, sau đó từ chối hoặc yêu cầu phê duyệt đúng hành động trước khi tác nhân lập trình có thể thực thi.',
    result: 'Từ chối, giữ hoặc cho phép trước khi thực thi.',
    demo: {
      ...SHELL_COMMAND_USE_CASE.demo,
      proposalTitle: 'Bash đề xuất một hành động phá hủy dữ liệu',
      proposalFields: [
        { label: 'Công cụ', value: 'claude-code / Bash' },
        { label: 'Workspace', value: '/workspace/project' },
      ],
      policyTitle: 'Chính sách công cụ khớp với dữ kiện của lệnh shell',
      policyFields: [
        { label: 'Rủi ro', value: 'filesystem_recursive_delete' },
        { label: 'Mục tiêu', value: 'root' },
        { label: 'Hành động', value: 'deny' },
      ],
      decisions: [
        { subject: 'rm -rf /', effect: 'deny', detail: 'Đã chặn mục tiêu hệ thống' },
        {
          subject: 'rm -rf ./build',
          effect: 'require_approval',
          detail: 'Chờ phê duyệt đúng hành động',
        },
      ],
      executionTitle: 'Bộ thực thi vẫn tạm dừng',
      executionDetail:
        'Lệnh bị từ chối sẽ không bao giờ chạy. Lệnh được phê duyệt nhận một giấy phép thực thi chỉ gắn với hành động đó.',
      boundary: 'Bộ phân tích đọc lệnh như dữ liệu có cấu trúc và không bao giờ gọi shell.',
    },
  },
  {
    ...EMAIL_USE_CASE,
    eyebrow: 'Hàng rào an toàn cho email gửi đi',
    title: 'Viết lại email rủi ro trước khi gửi.',
    summary:
      'Áp dụng chính sách nội dung cho email để bản nháp an toàn được giữ nguyên, còn cam kết rủi ro được thay bằng ngôn từ đã được chính sách phê duyệt trước khi gửi.',
    result: 'Cho phép bản nháp an toàn hoặc trả về bản viết lại đã được chính sách phê duyệt.',
    demo: {
      ...EMAIL_USE_CASE.demo,
      proposalTitle: 'Tác nhân đề xuất một email cho khách hàng',
      proposalCode: 'Đây là khoản hoàn tiền được đảm bảo.',
      proposalFields: [
        { label: 'Thao tác', value: 'send_email' },
        { label: 'Kênh', value: 'email' },
      ],
      policyTitle: 'Chính sách email kiểm tra bản nháp',
      policyFields: [
        { label: 'Nội dung khớp', value: 'guaranteed refund' },
        { label: 'Hành động', value: 'transform' },
        { label: 'Phạm vi', value: 'support-agent' },
      ],
      decisions: [
        { subject: 'Bản nháp an toàn', effect: 'permit', detail: 'Gửi không thay đổi' },
        {
          subject: 'Cam kết rủi ro',
          effect: 'transform',
          detail: 'Dùng nội dung thay thế an toàn',
        },
      ],
      executionTitle: 'Hệ thống gửi mail của khách hàng áp dụng kết quả',
      executionDetail:
        'Ứng dụng gửi bản nháp gốc đã được cho phép hoặc nội dung thay thế đã được chính sách phê duyệt.',
      boundary: 'TrustLoopGuard đánh giá tin nhắn được đề xuất và không bao giờ tự gửi email.',
    },
  },
  {
    ...AGENT_SPENDING_CAPS_USE_CASE,
    eyebrow: 'Hạn mức chi tiêu của tác nhân',
    title: 'Áp dụng hạn mức chi tiêu trước khi thanh toán.',
    summary:
      'Dùng một chính sách tài chính để cho phép khoản chi thông thường, giữ ngoại lệ để phê duyệt và từ chối thanh toán vượt trần cứng.',
    result: '$25 cho phép. $75 giữ lại. $150 từ chối.',
    demo: {
      ...AGENT_SPENDING_CAPS_USE_CASE.demo,
      proposalTitle: 'Tác nhân đề xuất thanh toán cho nhà cung cấp',
      proposalFields: [
        { label: 'Thao tác', value: 'pay_vendor' },
        { label: 'Chủ thể', value: 'spend-agent' },
      ],
      policyTitle: 'Chính sách tài chính kiểm tra thẩm quyền',
      policyFields: [
        { label: 'Mỗi hành động', value: 'Trần cứng $100' },
        { label: 'Phê duyệt trên mức', value: '$50' },
        { label: 'Hàng tháng', value: '$1,000' },
      ],
      decisions: [
        { subject: '$25 thông thường', effect: 'permit', detail: 'Đã cấp quyền' },
        { subject: '$75 ngoại lệ', effect: 'require_approval', detail: 'Giữ để xem xét' },
        { subject: '$150 vượt trần', effect: 'deny', detail: 'Đã chặn' },
      ],
      executionTitle: 'Lệnh gọi nhà cung cấp đang chờ',
      executionDetail:
        'Chỉ hành động đang được cấp quyền mới có thể giữ ngân sách khả dụng và đến nhà cung cấp thanh toán.',
      boundary: 'Quá trình phân tích cấp quyền không bao giờ thực thi thanh toán.',
    },
  },
] as const;

export function Why({ locale = 'en' }: { locale?: MarketingLocale }) {
  const isVietnamese = locale === 'vi';

  return (
    <section
      id="use-cases"
      aria-labelledby="use-cases-heading"
      className="section use-cases-section"
    >
      <div className="section-heading split-heading">
        <div>
          <p className="eyebrow">{isVietnamese ? 'Phê duyệt thực tế' : 'Approval patterns'}</p>
          <h2 id="use-cases-heading" className="section-title">
            {isVietnamese
              ? 'Bắt đầu với một hành động bạn dễ nhận biết.'
              : 'One approval layer. Every consequential action.'}
          </h2>
        </div>
        <p className="section-copy">
          {isVietnamese
            ? 'Chọn một hành động thực tế và theo dõi nó qua cùng một vòng kiểm soát: ghi nhận đề xuất, đánh giá chính sách bên ngoài prompt, trả về quyết định rõ ràng, rồi để runtime hiện có thực thi.'
            : 'Payments, hospital workflows, data access, and tool calls all follow the same contract: the agent proposes, policy evaluates, the right person approves when needed, and only then can execution begin.'}
        </p>
      </div>

      <UseCaseShowcase
        useCases={isVietnamese ? FEATURED_USE_CASES_VI : FEATURED_USE_CASES}
        locale={locale}
      />

      <Link href="/use-cases" className="use-case-showcase-all">
        {isVietnamese ? 'Xem tất cả sáu tình huống sử dụng' : 'View all six use cases'}{' '}
        <span aria-hidden="true">→</span>
      </Link>
    </section>
  );
}
