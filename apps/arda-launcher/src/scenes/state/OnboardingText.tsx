import { motion, AnimatePresence } from 'framer-motion'

const texts = [
  { minPhase: 1.0, text: 'Initializing Memory...' },
  { minPhase: 2.8, text: 'Discovering Local Services...' },
  { minPhase: 4.3, text: 'Loading Knowledge...' },
  { minPhase: 5.8, text: 'Building Your World...' },
  { minPhase: 8.2, text: 'ARDA' },
  { minPhase: 9.8, text: "Welcome.\nLet's build your environment." },
]

export default function OnboardingText({ phase }: { phase: number }) {
  const current = texts.findLast(t => phase >= t.minPhase)

  return (
    <div className="absolute bottom-[24%] left-1/2 -translate-x-1/2 z-40 pointer-events-none">
      <AnimatePresence mode="wait">
        {current && (
          <motion.div
            key={current.text}
            initial={{ opacity: 0, y: 15 }}
            animate={{ opacity: 0.85, y: 0 }}
            exit={{ opacity: 0, y: -12 }}
            transition={{ 
              duration: 0.7, 
              ease: [0.22, 1, 0.36, 1]   // smooth ease
            }}
            className="text-center"
          >
            <div className="font-mono tracking-[5px] text-[#f4e9d8] text-4xl md:text-[52px] whitespace-pre-line drop-shadow-[0_2px_20px_rgba(0,0,0,0.5)]">
              {current.text}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}