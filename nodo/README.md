
Types of channels:

- CX: Configuration is light-weight data which is read once during the startup phase and rarely changed.
- SX: State is light-weight data which is updated each step and captures the dynamic variables of a codelet. The initial state is created during the startup phase.
- RX: Messages are received during step and processed.
- TX: Messages are published during step and sent to other entities.

ATLAS is a tempo-spatial database which stores data associated with a timestamp and a coordinate frame. ATLAS allows read-only queries during codelet step. Changes to ATLAS have to be sent as messages. They are integrated into ATLAS during a special phase of exclusive access and only observeable after this phase. All schedules which access ATLAS need to provide a globally synced timeslot to allow for those updates.
