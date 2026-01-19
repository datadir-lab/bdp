'use client';

import { siteConfig } from '@/lib/site-config';

export function TimeEstimateNote() {
  return (
    <small style={{ opacity: 0.7, fontStyle: 'italic' }}>
      *Note: Time estimates are illustrative based on researcher interviews. We're actively collecting data on workflow inefficiencies. Have data or want to share your experience?{' '}
      <a href={`mailto:${siteConfig.contactEmail}`}>Contact us</a> or{' '}
      <a href={`${siteConfig.github.url}/discussions`} target="_blank" rel="noopener noreferrer">
        open a discussion
      </a>.*
    </small>
  );
}
