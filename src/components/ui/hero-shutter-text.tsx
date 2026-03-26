import { AnimatePresence, motion } from 'framer-motion';

interface HeroShutterTextProps {
  text: string;
  tone?: 'default' | 'success' | 'warning' | 'error';
  className?: string;
}

export function HeroShutterText({
  text,
  tone = 'default',
  className = '',
}: HeroShutterTextProps) {
  const characters = text.split('');

  return (
    <div
      className={`hero-shutter hero-shutter-${tone}${className ? ` ${className}` : ''}`}
      aria-hidden="true"
    >
      <AnimatePresence initial={false} mode="sync">
        <motion.div
          key={text}
          className="hero-shutter-word"
          initial={{ opacity: 0.18, filter: 'blur(10px)', scale: 0.985, y: 8 }}
          animate={{ opacity: 1, filter: 'blur(0px)', scale: 1, y: 0 }}
          exit={{ opacity: 0.46, filter: 'blur(5px)', scale: 1.012, y: -3 }}
          transition={{ duration: 0.52, ease: [0.22, 1, 0.36, 1] }}
        >
          {characters.map((char, index) => (
            <span key={`${text}-${index}`} className="hero-shutter-char-wrap">
              <motion.span
                initial={{ opacity: 0.24, filter: 'blur(6px)', y: 4 }}
                animate={{ opacity: 1, filter: 'blur(0px)' }}
                transition={{ delay: index * 0.035 + 0.08, duration: 0.48 }}
                className="hero-shutter-char hero-shutter-base"
              >
                {char === ' ' ? '\u00A0' : char}
              </motion.span>

              <motion.span
                initial={{ x: '-105%', opacity: 0 }}
                animate={{ x: '105%', opacity: [0, 0.95, 0] }}
                transition={{
                  duration: 0.62,
                  delay: index * 0.035 + 0.02,
                  ease: 'easeInOut',
                }}
                className="hero-shutter-char hero-shutter-slice hero-shutter-slice-top"
              >
                {char === ' ' ? '\u00A0' : char}
              </motion.span>

              <motion.span
                initial={{ x: '105%', opacity: 0 }}
                animate={{ x: '-105%', opacity: [0, 0.8, 0] }}
                transition={{
                  duration: 0.62,
                  delay: index * 0.035 + 0.1,
                  ease: 'easeInOut',
                }}
                className="hero-shutter-char hero-shutter-slice hero-shutter-slice-middle"
              >
                {char === ' ' ? '\u00A0' : char}
              </motion.span>

              <motion.span
                initial={{ x: '-105%', opacity: 0 }}
                animate={{ x: '105%', opacity: [0, 0.95, 0] }}
                transition={{
                  duration: 0.62,
                  delay: index * 0.035 + 0.18,
                  ease: 'easeInOut',
                }}
                className="hero-shutter-char hero-shutter-slice hero-shutter-slice-bottom"
              >
                {char === ' ' ? '\u00A0' : char}
              </motion.span>
            </span>
          ))}
        </motion.div>
      </AnimatePresence>
    </div>
  );
}
