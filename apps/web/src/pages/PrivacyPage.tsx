import { useI18n } from '@deliveryos/ui';
import { Link } from 'react-router-dom';

// Minimal /privacy notice (ADR-soft-access-gate, STOP-2). Must exist before the
// access-request CTA ships (the consent link must resolve — a link-to-404 would itself be
// a GDPR failure). Content: basis (consent), data, purpose, retention (12 months from
// first contact), rights, and a reachable erasure contact (Counsel R2 #2).
//
// The notice prose is bound to PRIVACY_NOTICE_VERSION by a CI content-hash test (R2-6):
// editing the copy without bumping the version fails the build.
export function PrivacyPage() {
  const { t } = useI18n();

  const Section = ({ title, body }: { title: string; body: string }) => (
    <section className="space-y-1">
      <h2 className="text-base font-semibold" style={{ color: 'var(--brand-text)' }}>{title}</h2>
      <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{body}</p>
    </section>
  );

  return (
    <div data-skin="paper" className="min-h-screen p-4" style={{ background: 'var(--brand-bg)', color: 'var(--brand-text)' }}>
      <div className="mx-auto max-w-xl py-8 space-y-6" data-testid="privacy-page">
        <h1 className="text-2xl font-bold" style={{ fontFamily: 'var(--brand-font-heading)' }}>
          {t('privacy.title', 'Privacy Notice')}
        </h1>
        <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>{t('privacy.intro', 'This notice explains how we handle the details you give us when you leave your email.')}</p>

        <Section title={t('privacy.basisTitle', 'Legal basis')} body={t('privacy.basis', 'Your explicit consent. You can withdraw it at any time.')} />
        <Section title={t('privacy.dataTitle', 'What we store')} body={t('privacy.data', 'Your email, a hashed code of your IP, and the time you consented.')} />
        <Section title={t('privacy.purposeTitle', 'Why')} body={t('privacy.purpose', 'Only to contact you about access and launch.')} />
        <Section title={t('privacy.retentionTitle', 'How long')} body={t('privacy.retention', 'For up to 12 months from when you first contact us, then it is deleted automatically.')} />
        <Section title={t('privacy.rightsTitle', 'Your rights')} body={t('privacy.rights', 'You can ask to see your data, delete it, or withdraw consent at any time.')} />

        <section className="space-y-1">
          <h2 className="text-base font-semibold" style={{ color: 'var(--brand-text)' }}>{t('privacy.contactTitle', 'Contact')}</h2>
          <p className="text-sm" style={{ color: 'var(--brand-text-muted)' }}>
            {t('privacy.contact', 'To delete your data or ask a question, email')}{' '}
            <a href={`mailto:${t('privacy.contactEmail', 'privacy@dowiz.org')}`} data-testid="privacy-erasure-contact" style={{ color: 'var(--brand-primary)', textDecoration: 'underline' }}>
              {t('privacy.contactEmail', 'privacy@dowiz.org')}
            </a>.
          </p>
        </section>

        <Link to="/" className="inline-block text-sm" style={{ color: 'var(--brand-primary)' }}>
          ← {t('privacy.back', 'Back')}
        </Link>
      </div>
    </div>
  );
}
