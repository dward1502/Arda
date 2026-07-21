import React from 'react'
import type { ArdaSourceProvenance } from '../../../lib/ardaProvenance'
import { DataFreshnessBadge } from './DataFreshnessBadge'
import { SourceRefreshAffordance } from './SourceRefreshAffordance'
import SourceActionContractPanel from './SourceActionContractPanel'

interface DataSourceDetailsPanelProps {
  record: ArdaSourceProvenance
  onClose: () => void
}

export const DataSourceDetailsPanel: React.FC<DataSourceDetailsPanelProps> = ({
  record,
  onClose,
}) => {
  const generatedAge = record.generatedAtUtc 
    ? Math.floor((new Date().getTime() - new Date(record.generatedAtUtc).getTime()) / 1000)
    : null
  
  const observedAge = record.observedAtUtc 
    ? Math.floor((new Date().getTime() - new Date(record.observedAtUtc).getTime()) / 1000)
    : null

  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50">
      <div className="bg-surface border border-slate-600 rounded-lg p-6 max-w-2xl max-h-96 overflow-y-auto">
        <div className="flex justify-between items-start mb-4">
          <h3 className="text-lg font-semibold">Source Details</h3>
          <button 
            onClick={onClose}
            className="text-slate-400 hover:text-slate-200 text-2xl"
            aria-label="Close"
          >
            x
          </button>
        </div>
        
        <div className="space-y-4">
          <div>
            <label className="text-sm font-medium text-slate-400">Domain</label>
            <p className="mt-1">{record.domainId}</p>
          </div>
          
          <div>
            <label className="text-sm font-medium text-slate-400">Label</label>
            <p className="mt-1">{record.label}</p>
          </div>
          
          <div>
            <label className="text-sm font-medium text-slate-400">State</label>
            <div className="mt-1">
              <DataFreshnessBadge record={record} />
            </div>
          </div>
          
          <div>
            <label className="text-sm font-medium text-slate-400">Source Paths</label>
            <ul className="mt-1 list-disc list-inside space-y-1">
              {record.sourcePaths.length > 0 ? (
                record.sourcePaths.map((path, index) => (
                  <li key={index} className="text-sm">{path}</li>
                ))
              ) : (
                <li className="text-sm text-slate-500">No source paths available</li>
              )}
            </ul>
          </div>

          <div>
            <label className="text-sm font-medium text-slate-400">Source Kind</label>
            <p className="mt-1">{record.sourceKind}</p>
          </div>

          {record.generatedAtUtc && (
            <div>
              <label className="text-sm font-medium text-slate-400">Generated At</label>
              <p className="mt-1">{record.generatedAtUtc} ({generatedAge} seconds ago)</p>
            </div>
          )}

          {record.observedAtUtc && (
            <div>
              <label className="text-sm font-medium text-slate-400">Observed At</label>
              <p className="mt-1">{record.observedAtUtc} ({observedAge} seconds ago)</p>
            </div>
          )}

          {record.derivedFrom && record.derivedFrom.length > 0 && (
            <div>
              <label className="text-sm font-medium text-slate-400">Derived From</label>
              <ul className="mt-1 list-disc list-inside space-y-1">
                {record.derivedFrom.map((source, index) => (
                  <li key={index} className="text-sm">{source}</li>
                ))}
              </ul>
            </div>
          )}
          
          <SourceRefreshAffordance record={record} />
          <SourceActionContractPanel record={record} />
          
          {record.lastRefreshResult && (
            <div>
              <label className="text-sm font-medium text-slate-400">Last Refresh Result</label>
              <p className={`mt-1 ${record.lastRefreshResult.success ? 'text-green-400' : 'text-red-400'}`}>
                {record.lastRefreshResult.success ? 'Success' : 'Failed'}: {record.lastRefreshResult.message}
              </p>
              <p className="text-xs text-slate-500">At {record.lastRefreshResult.timestampUtc}</p>
            </div>
          )}
          
          {record.notes && (
            <div>
              <label className="text-sm font-medium text-slate-400">Notes</label>
              <p className="mt-1">{record.notes}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
