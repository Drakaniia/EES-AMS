// Web NFC helpers. Only works on Android Chrome over HTTPS.

export type NfcSupport = 'supported' | 'unsupported';

export function nfcSupported(): NfcSupport {
	if (typeof window === 'undefined') return 'unsupported';
	return 'NDEFReader' in window ? 'supported' : 'unsupported';
}

type NDEFReadingEventLike = { serialNumber?: string };

export class NfcScanner {
	private reader: unknown = null;
	private controller: AbortController | null = null;
	private onRead: (serial: string) => void;
	private onError: (err: Error) => void;

	constructor(onRead: (serial: string) => void, onError: (err: Error) => void) {
		this.onRead = onRead;
		this.onError = onError;
	}

	async start() {
		if (!nfcSupported()) {
			this.onError(new Error('Web NFC not supported on this device/browser.'));
			return;
		}
		try {
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			this.reader = new (window as any).NDEFReader();
			this.controller = new AbortController();
			await (this.reader as any).scan({ signal: this.controller.signal });
			(this.reader as any).onreadingerror = () =>
				this.onError(new Error('Cannot read this card.'));
			(this.reader as any).onreading = (event: NDEFReadingEventLike) => {
				const serial = (event.serialNumber ?? '').toLowerCase().replace(/[^a-f0-9:]/g, '');
				if (serial) this.onRead(serial);
			};
		} catch (e: unknown) {
			this.onError(e instanceof Error ? e : new Error(String(e)));
		}
	}

	stop() {
		this.controller?.abort();
		this.controller = null;
		this.reader = null;
	}
}
