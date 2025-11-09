import csv

with open("./service_port.csv") as f:
    reader = csv.reader(f)
    records = []
    for row in reader:
        service_name = row[0]
        if len(service_name) == 0:
            continue
        if service_name.lower() == "discard":
            continue
        port_number = row[1]
        if len(port_number) == 0:
            continue
        transport_protocol = row[2]
        description = row[3]
        if description.lower() in ["discard", "reserved", "unassigned"]:
            continue
        records.append({
            "service_name": service_name,
            "port_number": port_number,
            "transport_protocol": transport_protocol,
            "description": description
        })
    records.pop(0)
    registered = set()
    registered_id = set()
    for r in records:
        service_name: str = r["service_name"]
        port_number: str = r["port_number"]
        port_str = port_number
        if port_number.count('-') > 0:
            ports = port_number.split('-')
            min_port = ports[0]
            max_port = ports[1]
            port_str = f"{min_port}, {max_port}"
        transport_protocol: str = r["transport_protocol"]
        description: str = r["description"]
        description = description.replace("\"", "\\\"")
        description_str = f"\"{description}\""
        service_name_str = f"\"{service_name}\""
        service_id = f"{service_name.upper()}_{transport_protocol.upper()}"\
            .replace('-', '_')\
            .replace('-', '')\
            .replace('-', '')\
            .replace('-', '')\
            .replace('/', '_')\
            .replace('+', 'P')\
            .replace('*', 'X')\
            .replace('.', '_')
        
        if service_id[0] >= '0' and service_id[0] <= '9':
            service_id = f"N_{service_id}"
        if f"{service_id}-{port_str}" in registered:
            continue
        registered.add(f"{service_id}-{port_str}")
        if service_id in registered_id:
            continue
        registered_id.add(service_id)
        print(f"define_service_port!({service_id}, {service_name_str}, {port_str}, {description_str});")
