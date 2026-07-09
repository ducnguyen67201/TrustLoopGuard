import Link from 'next/link';
import { BOOK_MEETING_URL, GITHUB_URL, getStarCount } from '@/lib/github';
import { NavActions } from './nav-actions';

const NAV_LINKS = [
  { href: '/#product', label: 'Product' },
  { href: '/#trust', label: 'Why trust us' },
  { href: '/#how', label: 'How it works' },
  { href: '/#developers', label: 'Developers' },
] as const;

export async function Nav() {
  const stars = await getStarCount();

  return (
    <header className="site-header sticky top-0 inset-x-0 z-40">
      <nav
        aria-label="Main navigation"
        className="site-nav"
      >
        <Link href="/" className="wordmark" aria-label="TrustLoopGuard home">
          <img src="/trustloop-logo.svg" alt="" aria-hidden="true" className="wordmark-logo" />
          <span>TrustLoopGuard</span>
        </Link>
        <ul className="site-nav-links">
          {NAV_LINKS.map((link) => (
            <li key={link.href}>
              <a href={link.href}>{link.label}</a>
            </li>
          ))}
        </ul>
        <NavActions bookMeetingUrl={BOOK_MEETING_URL} githubUrl={GITHUB_URL} stars={stars} />
      </nav>
    </header>
  );
}
