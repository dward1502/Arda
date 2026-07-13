import { motion, AnimatePresence } from 'framer-motion'

export default function ArdaLogo({ show }: { show: boolean }) {
  return (
    <AnimatePresence>
      {show && (
        <motion.div
          initial={{ opacity: 0, scale: 0.6 }}
          animate={{ opacity: 1, scale: 1 }}
          exit={{ opacity: 0, scale: 1.15 }}
          transition={{ duration: 0.7, ease: [0.22, 1, 0.36, 1] }}
          className="absolute inset-0 flex items-center justify-center z-40 pointer-events-none"
        >
          <div className="relative">
            <div className="text-[92px] font-light tracking-[12px] text-[#ffe070] drop-shadow-[0_0_40px_#c5a26f]">
              ARDA
            </div>
            <motion.div
              animate={{ opacity: [0.3, 1, 0.3] }}
              transition={{ duration: 1.8, repeat: Infinity }}
              className="absolute inset-0 bg-gradient-to-b from-transparent via-[#ffe070]/10 to-transparent blur-3xl"
            />
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}