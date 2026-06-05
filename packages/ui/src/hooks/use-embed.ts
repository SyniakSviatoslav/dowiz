import { useState, useEffect } from 'react';

export function useEmbed(): boolean {
  const [embed, setEmbed] = useState(false);

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    setEmbed(params.get('embed') === 'true');
  }, []);

  return embed;
}
