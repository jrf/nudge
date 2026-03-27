import EventKit
import Foundation

nonisolated(unsafe) let store = EKEventStore()

func requestAccess() -> Bool {
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var granted = false
    if #available(macOS 14.0, *) {
        store.requestFullAccessToReminders { g, _ in
            granted = g
            sem.signal()
        }
    } else {
        store.requestAccess(to: .reminder) { g, _ in
            granted = g
            sem.signal()
        }
    }
    sem.wait()
    return granted
}

func fmtDate(_ date: Date?) -> String {
    guard let date else { return "" }
    let f = DateFormatter()
    f.dateFormat = "yyyy-MM-dd"
    return f.string(from: date)
}

func listLists() {
    let calendars = store.calendars(for: .reminder)
    for cal in calendars {
        print(cal.title)
    }
}

func listReminders(listFilter: String?, showCompleted: Bool) {
    let calendars: [EKCalendar]
    if let filter = listFilter {
        calendars = store.calendars(for: .reminder).filter { $0.title == filter }
    } else {
        calendars = store.calendars(for: .reminder)
    }

    let predicate = store.predicateForReminders(in: calendars)
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var results: [EKReminder] = []
    store.fetchReminders(matching: predicate) { reminders in
        results = reminders ?? []
        sem.signal()
    }
    sem.wait()

    for r in results {
        if !showCompleted && r.isCompleted { continue }
        let list = r.calendar.title
        let id = r.calendarItemIdentifier
        let name = r.title ?? ""
        let due = fmtDate(r.dueDateComponents?.date)
        let completed = r.isCompleted
        let priority: Int = r.priority
        print("\(list)|||\(id)|||\(name)|||\(due)|||\(completed)|||\(priority)")
    }
}

func searchReminders(query: String) {
    let predicate = store.predicateForReminders(in: nil)
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var results: [EKReminder] = []
    store.fetchReminders(matching: predicate) { reminders in
        results = reminders ?? []
        sem.signal()
    }
    sem.wait()

    let q = query.lowercased()
    for r in results {
        let name = r.title ?? ""
        guard name.lowercased().contains(q) else { continue }
        let list = r.calendar.title
        let id = r.calendarItemIdentifier
        let due = fmtDate(r.dueDateComponents?.date)
        let completed = r.isCompleted
        let priority: Int = r.priority
        print("\(list)|||\(id)|||\(name)|||\(due)|||\(completed)|||\(priority)")
    }
}

func addReminder(name: String, listName: String?, dueDate: String?, priority: Int?) {
    let calendar: EKCalendar
    if let listName {
        guard let found = store.calendars(for: .reminder).first(where: { $0.title == listName })
        else {
            fputs("Error: list '\(listName)' not found\n", stderr)
            exit(1)
        }
        calendar = found
    } else {
        calendar = store.defaultCalendarForNewReminders()!
    }

    let reminder = EKReminder(eventStore: store)
    reminder.title = name
    reminder.calendar = calendar

    if let priority {
        reminder.priority = priority
    }

    if let dueDate {
        let f = DateFormatter()
        f.dateStyle = .medium
        f.timeStyle = .none
        if let date = f.date(from: dueDate) {
            reminder.dueDateComponents = Calendar.current.dateComponents(
                [.year, .month, .day], from: date)
        } else {
            let iso = DateFormatter()
            iso.dateFormat = "yyyy-MM-dd"
            if let date = iso.date(from: dueDate) {
                reminder.dueDateComponents = Calendar.current.dateComponents(
                    [.year, .month, .day], from: date)
            }
        }
    }

    try! store.save(reminder, commit: true)
    print("ok")
}

func completeReminder(name: String) {
    let predicate = store.predicateForReminders(in: nil)
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var results: [EKReminder] = []
    store.fetchReminders(matching: predicate) { reminders in
        results = reminders ?? []
        sem.signal()
    }
    sem.wait()

    let q = name.lowercased()
    guard
        let r = results.first(where: {
            !$0.isCompleted && ($0.title ?? "").lowercased().contains(q)
        })
    else {
        fputs("Error: no incomplete reminder matching '\(name)'\n", stderr)
        exit(1)
    }

    r.isCompleted = true
    try! store.save(r, commit: true)
    print("ok")
}

func deleteReminder(name: String) {
    let predicate = store.predicateForReminders(in: nil)
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var results: [EKReminder] = []
    store.fetchReminders(matching: predicate) { reminders in
        results = reminders ?? []
        sem.signal()
    }
    sem.wait()

    let q = name.lowercased()
    guard let r = results.first(where: { ($0.title ?? "").lowercased().contains(q) }) else {
        fputs("Error: no reminder matching '\(name)'\n", stderr)
        exit(1)
    }

    try! store.remove(r, commit: true)
    print("ok")
}

func moveReminder(id: String, toList: String) {
    guard let calendar = store.calendars(for: .reminder).first(where: { $0.title == toList }) else {
        fputs("Error: list '\(toList)' not found\n", stderr)
        exit(1)
    }

    let predicate = store.predicateForReminders(in: nil)
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var results: [EKReminder] = []
    store.fetchReminders(matching: predicate) { reminders in
        results = reminders ?? []
        sem.signal()
    }
    sem.wait()

    guard let r = results.first(where: { $0.calendarItemIdentifier == id }) else {
        fputs("Error: no reminder with id '\(id)'\n", stderr)
        exit(1)
    }

    r.calendar = calendar
    try! store.save(r, commit: true)
    print("ok")
}

func createList(name: String) {
    let calendar = EKCalendar(for: .reminder, eventStore: store)
    calendar.title = name
    calendar.source = store.defaultCalendarForNewReminders()!.source
    try! store.saveCalendar(calendar, commit: true)
    print("ok")
}

func renameList(oldName: String, newName: String) {
    guard let calendar = store.calendars(for: .reminder).first(where: { $0.title == oldName }) else {
        fputs("Error: list '\(oldName)' not found\n", stderr)
        exit(1)
    }
    calendar.title = newName
    try! store.saveCalendar(calendar, commit: true)
    print("ok")
}

func deleteList(name: String) {
    guard let calendar = store.calendars(for: .reminder).first(where: { $0.title == name }) else {
        fputs("Error: list '\(name)' not found\n", stderr)
        exit(1)
    }
    try! store.removeCalendar(calendar, commit: true)
    print("ok")
}

func uncompleteReminder(id: String) {
    let predicate = store.predicateForReminders(in: nil)
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var results: [EKReminder] = []
    store.fetchReminders(matching: predicate) { reminders in
        results = reminders ?? []
        sem.signal()
    }
    sem.wait()

    guard let r = results.first(where: { $0.calendarItemIdentifier == id }) else {
        fputs("Error: no reminder with id '\(id)'\n", stderr)
        exit(1)
    }

    r.isCompleted = false
    r.completionDate = nil
    try! store.save(r, commit: true)
    print("ok")
}

func editReminder(id: String, newName: String) {
    let predicate = store.predicateForReminders(in: nil)
    let sem = DispatchSemaphore(value: 0)
    nonisolated(unsafe) var results: [EKReminder] = []
    store.fetchReminders(matching: predicate) { reminders in
        results = reminders ?? []
        sem.signal()
    }
    sem.wait()

    guard let r = results.first(where: { $0.calendarItemIdentifier == id }) else {
        fputs("Error: no reminder with id '\(id)'\n", stderr)
        exit(1)
    }

    r.title = newName
    try! store.save(r, commit: true)
    print("ok")
}

// --- Main ---

guard requestAccess() else {
    fputs("Error: Reminders access denied\n", stderr)
    exit(1)
}

let args = Array(CommandLine.arguments.dropFirst())
guard let command = args.first else {
    fputs("Usage: nudge-bridge <command> [args...]\n", stderr)
    exit(1)
}

switch command {
case "lists":
    listLists()
case "list":
    var listFilter: String?
    var showCompleted = false
    var i = 1
    while i < args.count {
        switch args[i] {
        case "--list": i += 1; listFilter = args[i]
        case "--all": showCompleted = true
        default: break
        }
        i += 1
    }
    listReminders(listFilter: listFilter, showCompleted: showCompleted)
case "search":
    guard args.count > 1 else {
        fputs("Usage: nudge-bridge search <query>\n", stderr)
        exit(1)
    }
    searchReminders(query: args[1])
case "add":
    guard args.count > 1 else {
        fputs(
            "Usage: nudge-bridge add <name> [--list <list>] [--due <date>] [--priority <n>]\n",
            stderr)
        exit(1)
    }
    let name = args[1]
    var listName: String?
    var dueDate: String?
    var priority: Int?
    var i = 2
    while i < args.count {
        switch args[i] {
        case "--list": i += 1; listName = args[i]
        case "--due": i += 1; dueDate = args[i]
        case "--priority": i += 1; priority = Int(args[i])
        default: break
        }
        i += 1
    }
    addReminder(name: name, listName: listName, dueDate: dueDate, priority: priority)
case "complete":
    guard args.count > 1 else {
        fputs("Usage: nudge-bridge complete <name>\n", stderr)
        exit(1)
    }
    completeReminder(name: args[1])
case "delete":
    guard args.count > 1 else {
        fputs("Usage: nudge-bridge delete <name>\n", stderr)
        exit(1)
    }
    deleteReminder(name: args[1])
case "move":
    guard args.count > 3, args[2] == "--list" else {
        fputs("Usage: nudge-bridge move <id> --list <list>\n", stderr)
        exit(1)
    }
    moveReminder(id: args[1], toList: args[3])
case "create-list":
    guard args.count > 1 else {
        fputs("Usage: nudge-bridge create-list <name>\n", stderr)
        exit(1)
    }
    createList(name: args[1])
case "rename-list":
    guard args.count > 2 else {
        fputs("Usage: nudge-bridge rename-list <old> <new>\n", stderr)
        exit(1)
    }
    renameList(oldName: args[1], newName: args[2])
case "delete-list":
    guard args.count > 1 else {
        fputs("Usage: nudge-bridge delete-list <name>\n", stderr)
        exit(1)
    }
    deleteList(name: args[1])
case "uncomplete":
    guard args.count > 1 else {
        fputs("Usage: nudge-bridge uncomplete <id>\n", stderr)
        exit(1)
    }
    uncompleteReminder(id: args[1])
case "edit":
    guard args.count > 2 else {
        fputs("Usage: nudge-bridge edit <id> <new-name>\n", stderr)
        exit(1)
    }
    editReminder(id: args[1], newName: args[2])
default:
    fputs("Unknown command: \(command)\n", stderr)
    exit(1)
}
